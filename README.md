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