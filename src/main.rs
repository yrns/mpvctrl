use serde_json::{from_str, Value};
use smol::{net::unix::UnixStream, prelude::*, process::Command};

const CMD: &[u8] =
    "{ \"command\": [\"observe_property_string\", 1, \"media-title\"] }\n".as_bytes();

const RETRY: usize = 3;

async fn connect() -> Result<UnixStream, std::io::Error> {
    let mut i = 0;
    loop {
        let r = UnixStream::connect("/tmp/mpvsocket").await;
        if r.is_ok() || i == RETRY - 1 {
            return r;
        }
        smol::Timer::after(std::time::Duration::from_secs(1)).await;
        i += 1;
    }
}

fn main() -> anyhow::Result<()> {
    smol::block_on(async {
        let out = Command::new("tmux")
            .arg("display-message")
            .arg("-p")
            .arg("#I")
            .output()
            .await?;
        let current_window = std::str::from_utf8(&out.stdout).unwrap().trim();

        let mut stream = connect().await?;

        stream.write_all(CMD).await?;
        let mut buf = vec![0u8; 1024];

        loop {
            let n = stream.read(&mut buf).await?;

            // mpv exited
            if n == 0 {
                break;
            }

            let cmds = std::str::from_utf8(&buf[..n]).unwrap();

            // parse each line separately, the newline is required (split_inclusive is nightly)
            for cmd in cmds.lines() {
                let mut s = cmd.to_string();
                s.push('\n');

                let v: serde_json::Result<Value> = from_str(&s);

                match v {
                    Ok(v) => {
                        if v["event"] == "property-change" {
                            let v = v["data"].as_str().unwrap();

                            let _ = match Command::new("tmux")
                                .arg("rename-window")
                                .arg("-t")
                                .arg(current_window)
                                .arg(v)
                                .output()
                                .await
                            {
                                Ok(_) => (),
                                Err(e) => eprintln!("error in command: {}", e),
                            };
                        }
                    }
                    Err(e) => eprintln!(
                        "error in json: '{}' -- {}",
                        e,
                        std::str::from_utf8(&buf[..n]).unwrap()
                    ),
                }
            }
        }

        Ok(())
    })
}
