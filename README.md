# amargo :coffee: :weary:

An `unsafe` experiment.

```rust
fn main() {
    let x: &i32 = {
      let local = 7;
      &local
    };
    println!("Hello, world! {:?}", *x);
}
```

```
$ amargo run --release
Hello, world! 0
```

## How does it work?
`amargo` is a wrapper around `cargo` that transpiles Rust code, allowing you to effectively write
in a Rust-looking version of C.