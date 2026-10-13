//! "Only allow one VLC window": a second launch hands its files to the
//! running instance over a Unix socket in the data directory, then exits.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use crossbeam_channel::Sender;

fn socket(data_dir: &Path) -> PathBuf {
    data_dir.join("instance.sock")
}

/// Sends `paths` to a running instance. Returns false when none is running.
pub fn forward(data_dir: &Path, paths: &[PathBuf]) -> bool {
    let Ok(mut stream) = UnixStream::connect(socket(data_dir)) else { return false };
    let mut msg = String::new();
    for p in paths {
        msg.push_str(&p.to_string_lossy());
        msg.push('\n');
    }
    // An empty message still brings the running window forward.
    msg.push('\n');
    stream.write_all(msg.as_bytes()).is_ok()
}

/// Listens for other launches and sends the paths each one was given
/// (possibly none) to `tx`.
pub fn listen(data_dir: &Path, tx: Sender<Vec<PathBuf>>) -> Option<()> {
    let path = socket(data_dir);
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).ok()?;
    std::thread::Builder::new()
        .name("single-instance".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut paths = Vec::new();
                for line in BufReader::new(stream).lines().map_while(Result::ok) {
                    if line.is_empty() {
                        break;
                    }
                    paths.push(PathBuf::from(line));
                }
                if tx.send(paths).is_err() {
                    return;
                }
            }
        })
        .ok()?;
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_launch_forwards_paths() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!forward(dir.path(), &[PathBuf::from("/a.mkv")]));
        let (tx, rx) = crossbeam_channel::unbounded();
        listen(dir.path(), tx).unwrap();
        assert!(forward(dir.path(), &[PathBuf::from("/a.mkv"), PathBuf::from("/b c.mp3")]));
        let got = rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        assert_eq!(got, vec![PathBuf::from("/a.mkv"), PathBuf::from("/b c.mp3")]);
    }
}
