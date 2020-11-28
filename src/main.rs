use serde_json::{from_slice, Value};
use smol::{net::unix::UnixStream, prelude::*, process::Command};

const CMD: &[u8] =
    "{ \"command\": [\"observe_property_string\", 1, \"media-title\"] }\n".as_bytes();

fn main() -> anyhow::Result<()> {
    smol::block_on(async {
        let mut buf = vec![0u8; 1024];
        let mut stream = UnixStream::connect("/tmp/mpvsocket").await?;
        stream.write_all(CMD).await?;

        loop {
            let n = stream.read(&mut buf).await?;
            let v: Value = from_slice(&buf[..n])?;

            if v["event"] == "property-change" {
                let v = v["data"].as_str().unwrap();

                let _ = Command::new("tmux")
                    .arg("rename-window")
                    .arg(v)
                    .output()
                    .await?;
            }
        }

        //Ok(())
    })
}
