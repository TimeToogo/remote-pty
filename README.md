Remote PTY

TODO:
- [x] implement tcsetattr
- [x] implement ioctl
- [x] filter fds to those specified from env
- [x] forward calls to libc/musl implementations if not in filtered fds
- [x] implement master/server handler for pty requests
- [x] consolidate stdin/out/err through remote PTY channel
- [x] shared library mode for child processes
- [x] SIGWINCH|SIG* handling (requires listener on slave side)
- [x] process group control and SIGTTOU/SIGTTIN handling
- [x] filter pty by pipe inodes rather than fd numbers
- [ ] cross compile for arm and strip binaries
- [ ] ci/cd to s3
- [ ] integrate into tunshell
- [ ] README
- [ ] better cross platform termios?