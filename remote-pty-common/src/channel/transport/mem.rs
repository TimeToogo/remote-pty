use std::{
    cmp::min,
    io::{self, Read, Write},
    sync::mpsc::{channel, Receiver, Sender},
};

use super::Transport;

pub struct MemoryTransport {
    tx: Tx,
    rx: Rx,
}

pub struct Tx(Sender<Vec<u8>>);
pub struct Rx(Receiver<Vec<u8>>, Vec<u8>);

impl MemoryTransport {
    pub fn pair() -> (Self, Self) {
        let (tx1, rx1) = channel();
        let (tx2, rx2) = channel();

        (
            Self {
                tx: Tx(tx1),
                rx: Rx(rx2, vec![]),
            },
            Self {
                tx: Tx(tx2),
                rx: Rx(rx1, vec![]),
            },
        )
    }
}

impl Transport for MemoryTransport {
    fn split(self) -> (Box<dyn Read + Send>, Box<dyn Write + Send>) {
        (Box::new(self.rx), Box::new(self.tx))
    }
}

impl Write for Tx {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .send(buf.to_vec())
            .map_err(|_| io::Error::from(io::ErrorKind::BrokenPipe))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Read for Rx {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        while self.1.is_empty() {
            self.1 = self
                .0
                .recv()
                .map_err(|_| io::Error::from(io::ErrorKind::BrokenPipe))?;
        }

        let idx = min(buf.len(), self.1.len());
        let remaining = self.1.split_off(idx);
        buf[..idx].copy_from_slice(self.1.as_slice());
        self.1 = remaining;

        Ok(idx)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};

    use super::MemoryTransport;

    #[test]
    fn test_io() {
        let (mut t1, mut t2) = MemoryTransport::pair();

        t1.tx.write(&[1, 2, 3, 4, 5]).unwrap();
        t1.tx.write(&[6, 7, 8]).unwrap();
        t1.tx.write(&[9, 10]).unwrap();

        t2.tx.write(&[1]).unwrap();
        t2.tx.write(&[2]).unwrap();
        t2.tx.write(&[3]).unwrap();

        let mut buf = [0u8; 32];

        t2.rx.read_exact(&mut buf[..7]).unwrap();
        assert_eq!(buf[..7], [1, 2, 3, 4, 5, 6, 7]);

        t2.rx.read_exact(&mut buf[..3]).unwrap();
        assert_eq!(buf[..3], [8, 9, 10]);

        t1.rx.read_exact(&mut buf[..3]).unwrap();
        assert_eq!(buf[..3], [1, 2, 3]);
    }
}
