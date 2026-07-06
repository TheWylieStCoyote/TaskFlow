# Rust examples

The Rust pack has language id `rust`, so `--lang` matches it by that id, by the `rs` file extension, or by the `rust` LSP language id; expansions indent with 4 spaces. See the [language reference](../languages.md) for the complete trigger list.

> Examples use the default `strip` mode, which renders plain code and leaves an empty spot wherever a tabstop will be — those are the places you fill in. Add `--tabstops keep` to see them as `$1` / `${1:placeholder}` / `$0` markers. See the [examples overview](README.md) for more.

## A struct with derived traits

```sh
codemmet expand --lang rust "derive+struct:Point"
```

```rust
#[derive(Debug, Clone)]
struct Point {
    
}
```

The `+` sibling operator places two expansions one after another: `derive` emits the `#[derive(...)]` attribute and `struct:Point` emits the struct body. The `:Point` is a glued argument that names the struct.

## An impl block with several methods

```sh
codemmet expand --lang rust "impl:Counter>fn:increment>pr^fn:value>ret"
```

```rust
impl Counter {
    fn increment(&self) {
        println!("");
    }
    fn value(&self) {
        return ;
    }
}
```

The `>` child operator nests `fn:increment` inside the `impl` block, and both methods gain `&self` because a `fn` composed inside an `impl` selects the method variant — that is context awareness through composition. The `^` climb-up operator makes `fn:value` a sibling of the first method rather than a child nested inside it.

## A trait with two methods

```sh
codemmet expand --lang rust "trait:Draw>fn:draw+fn:bounds"
```

```rust
trait Draw {
    fn draw(&self) {
        
    }
    fn bounds(&self) {
        
    }
}
```

Both methods gain `&self` from the same context awareness that applies inside an `impl`. The `+` sibling operator keeps `fn:draw` and `fn:bounds` at the same level inside the trait.

## Matching on a Result

```sh
codemmet expand --lang rust "fn:load>matchres"
```

```rust
fn load() {
    match expr {
        Ok(value) => ,
        Err(err) => ,
    }
}
```

The `matchres` trigger expands to a full `match` on a `Result`, with `Ok` and `Err` arms already written. The `>` child operator nests it inside the function body.

## Deep nesting, then climbing out

```sh
codemmet expand --lang rust "fn:process>fori10>ifl>prf:\"item {}\",value^^ret"
```

```rust
fn process() {
    for i in 0..10 {
        if let Some(value) = expr {
            println!("item {}", value);
        }
    }
    return ;
}
```

Three `>` operators nest the expansion three levels deep: a counted `for` loop, an `if let`, and a formatted print. The `prf` trigger takes explicit arguments — the format string `"item {}"` and `value` are passed after the colon and separated by a comma. The `^^` climb-up operator climbs two levels so the `return` lands back in the function body.

## A test module with two tests

```sh
codemmet expand --lang rust "tmod>test:adds>aeq^test:handles_empty>aeq"
```

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds() {
        assert_eq!(left, );
    }
    #[test]
    fn handles_empty() {
        assert_eq!(left, );
    }
}
```

The `tmod` trigger emits a `#[cfg(test)]` module with the `use super::*;` import. Each `test:<name>` produces a `#[test]` function with a glued name argument, and `aeq` fills in an `assert_eq!`. The `^` climb-up operator makes the second test a sibling of the first.

## Seeing the tabstops

```sh
codemmet expand --lang rust --tabstops keep "impl:Point>fn:new>ret"
```

```rust
impl Point {
    fn new(&self$1) {
        return $0;
    }
}
```

With `--tabstops keep` the empty spots become markers: `$1` is the first stop, placed right after `&self` where you add more parameters, and `$0` is the final cursor position after `return`. codemmet renumbers tabstops across the whole composed expansion, so the numbers stay sequential no matter how many triggers you combine.

## See also

- [abbreviation syntax](../syntax.md)
- [language reference](../languages.md)
- [examples overview](README.md)
