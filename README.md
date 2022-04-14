Remote PTY

TODO:
- [x] implement tcsetattr
- [x] implement ioctl
- [x] filter fds to those specified from env
- [x] forward calls to libc/musl implementations if not in filtered fds
- [x] implement master/server handler for pty requests
- [ ] consolidate stdin/out/err through remote PTY channel
- [ ] SIGWINCH|SIG* handling (requires listener on slave side)
- [ ] shared library mode for child processes
- [ ] process group control and SIGTTOU/SIGTTIN handling