Remote Pseudoterminals
======================

Remote Pseudoterminals or "RPTY" is a [Rust](https://www.rust-lang.org/) library which intercepts calls to the Linux kernel's 
TTY/PTY-related libc functions and executes them on a remote host. Thereby emulating the presence of a local tty kernel object that is
operating outside of the local kernel.

It also includes a patched bash shell which is statically linked against the RPTY library and [busybox](https://busybox.net) builtins.

**⚠️ Warning:** Please do not put this library anywhere near production. It is full of unsafe, unethical, cursed code that occasionally works for inexplicable reasons.

## Why?

> Why does this library exist?

For very bad reasons. I built [Tunshell](https://tunshell.com) with the goal of easily gaining shell access to ephemeral
environments such as AWS Lambda. Similarly to SSH, Tunshell spawns a shell on the remote host and streams it to your local terminal.
However, the AWS Lambda environment is _very_ locked down and disables kernel support for creating PTY's thereby making most shells non-interactive.

RPTY goes to extreme lengths (such as intercepting libc functions, overriding stdin/out/err) in order to workaround this restriction and provide a nicer shell experience.

I realised half-way through this project that the "right" way to solve this problem would be by building a client-server shell (not SSH, which is not a shell). 
A shell where all the interactive elements run on the client and the remote server accepts commands over a socket. 
However I could not find such a shell. I'll leave this as an exercise to the reader.

## How does it work?

For an overview of the TTY/PTY Linus Åkesson has written a [superb article](https://www.linusakesson.net/programming/tty/) that helped me get my head around it. 

The best way to understand how RPTY works is to compare it with more familiar constructs.

### Local

Take the simple case where you have a local terminal emulator and shell: 

![Local shell diagram](https://lucid.app/publicSegments/view/0efb720a-93e9-456b-a644-0e5cc85ffe62/image.png)


 We can see the PTY is essentially a bidirectional pipe/socket between the terminal and the shell. It performs buffering and processing to provide line-editing and other features. The behavior can be modified by updating the settings (termios). 

### SSH

In the case of SSH we have a PTY instance on both the client and server:

![SSH remote shell diagram](https://lucid.app/publicSegments/view/988105ad-dd25-43a7-97c8-80be52a6f9b0/image.png)

The local ssh client puts the local PTY into "raw mode" disabling all buffering and processing becoming a pass-through.

SSHD creates a PTY instance on the remote host. The PTY on the remote provides the line-editing and interactive features. SSHD spawns a shell and connects its stdin/stdout/stderr to the slave side of the PTY.

### RPTY 

RPTY removes the requirement of having a PTY instance on the remote host:

![RPTY remote shell diagram](https://lucid.app/publicSegments/view/6abe4033-c33a-4ef0-ae60-84a7f7eccc3c/image.png)

The local client (RPTY master) leaves the local PTY settings in-tact. The remote process (RPTY slave) forwards all TTY-related libc function calls from the remote shell to the local client.
From the perspective of the remote shell, the function calls are synchronous and blocking which mimics the behavior of native calls into libc.

## Modes of operation

In order for RPTY to work it has to be able to be able to intercept calls made to libc functions. There are two supported approaches:

### Static-linking

RPTY can be built as a static-archive linked against musl. This archive then needs to be statically linked against the shell at build time.
This approach removes runtime dependencies as it is compiled into the shell binary. However, it does not work with exec'd processes making the shell much less useful. This is somewhat mitigated by the bash build script in this repo which statically links to [busybox](https://busybox.net).

### Dynamic linking

RPTY can also be built against GNU libc as a shared library. Then it can be injected into any shell at runtime via `LD_PRELOAD`. This approach should work with any exec'd processes which inherit the `LD_PRELOAD` environment variable.

## Supported targets

| Master | Slave |
|--------|-------|
| x86_64-unknown-linux-musl | x86_64-unknown-linux-musl |
| x86_64-unknown-linux-gnu | x86_64-unknown-linux-gnu |
| x86_64-apple-darwin | |

See [latest actions](https://github.com/TimeToogo/remote-pty/actions) to find the latest build of the libraries.