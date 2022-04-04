# Rac

A simple MAC address utility written in Rust.

## Notes

**Only works on linux right now! (may change in the future)**

## Usage

It can be used to:

- Change your MAC address to a random or specified one
- Generate a random MAC address
- Show your current MAC address

Use `rac set -r` to change your MAC address to a random one.

**Full cmdline help:**

```sh
mac 0.1.0
A simple  MAC address utility

USAGE:
    mac [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -c, --current    Print current MAC address
    -h, --help       Print help information
    -r, --random     Generate a random MAC address
    -V, --version    Print version information

SUBCOMMANDS:
    help    Print this message or the help of the given subcommand(s)
    set
```

## License

Under the [MIT Licence](https://choosealicense.com/licenses/mit/)
