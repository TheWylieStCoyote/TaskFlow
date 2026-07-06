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

## Trigger reference

Every Rust trigger, in alphabetical order. Each entry shows the abbreviation and its expansion in the default `strip` mode.

### `aeq`

assert_eq! statement.

```sh
codemmet expand --lang rust "aeq"
```

```rust
assert_eq!(left, );
```

Emits an `assert_eq!` comparing two values. The empty spot after `left,` is where the expected value goes.

### `afn`

async function definition.

```sh
codemmet expand --lang rust "afn:fetch"
```

```rust
async fn fetch() {
    
}
```

```sh
codemmet expand --lang rust "afn:fetch>pr"
```

```rust
async fn fetch() {
    println!("");
}
```

Produces an `async fn` with a glued name argument (`:fetch`). Nesting a child with `>` places statements inside the body.

### `arm`

match arm.

```sh
codemmet expand --lang rust "arm"
```

```rust
pattern => ,
```

A single `pattern => expr` match arm. Multiply it with `*n` (e.g. `arm*2`) to emit several arms at once.

### `cl`

closure.

```sh
codemmet expand --lang rust "cl"
```

```rust
|args| 
```

Emits a closure with an `args` parameter list and an empty body spot for the expression.

### `const`

const item.

```sh
codemmet expand --lang rust "const:MAX"
```

```rust
const MAX: Type = ;
```

A `const` item with a glued name argument (`:MAX`). The `Type` and value are left as spots to fill.

### `derive`

derive attribute.

```sh
codemmet expand --lang rust "derive"
```

```rust
#[derive(Debug, Clone)]
```

A `#[derive(...)]` attribute prefilled with `Debug, Clone`; edit the trait list as needed.

### `doc`

doc comment line.

```sh
codemmet expand --lang rust "doc"
```

```rust
/// 
```

A single `///` documentation comment line, with the cursor left after the slashes.

### `el`

else block.

```sh
codemmet expand --lang rust "el"
```

```rust
else {
    
}
```

```sh
codemmet expand --lang rust "el>pr"
```

```rust
else {
    println!("");
}
```

An `else` block. Nesting a child with `>` fills the body.

### `elif`

else-if block.

```sh
codemmet expand --lang rust "elif"
```

```rust
else if condition {
    
}
```

```sh
codemmet expand --lang rust "elif>pr"
```

```rust
else if condition {
    println!("");
}
```

An `else if` block with a `condition` spot. Nesting a child with `>` fills the body.

### `enum`

enum definition.

```sh
codemmet expand --lang rust "enum:Color"
```

```rust
enum Color {
    
}
```

```sh
codemmet expand --lang rust "enum:Color>doc"
```

```rust
enum Color {
    /// 
}
```

An `enum` with a glued name argument (`:Color`). Nesting with `>` places items such as a doc comment inside the body.

### `epr`

eprintln! to stderr.

```sh
codemmet expand --lang rust "epr"
```

```rust
eprintln!("");
```

An `eprintln!` that writes to standard error, with an empty format string to fill.

### `fn`

function definition.

```sh
codemmet expand --lang rust "fn:greet"
```

```rust
fn greet() {
    
}
```

```sh
codemmet expand --lang rust "fn:greet>pr"
```

```rust
fn greet() {
    println!("");
}
```

A free function with a glued name argument (`:greet`). Composed inside an `impl` or `trait` it gains a `&self` receiver; nesting a child with `>` fills the body.

### `fore`

for-each loop over an iterator.

```sh
codemmet expand --lang rust "fore"
```

```rust
for item in iterable {
    
}
```

```sh
codemmet expand --lang rust "fore>pr"
```

```rust
for item in iterable {
    println!("");
}
```

A `for item in iterable` loop over an iterator. Nesting a child with `>` fills the body.

### `fori`

for loop over integer range.

```sh
codemmet expand --lang rust "fori10"
```

```rust
for i in 0..10 {
    
}
```

```sh
codemmet expand --lang rust "fori10>pr"
```

```rust
for i in 0..10 {
    println!("");
}
```

A counted `for i in 0..N` loop; the number glued to the trigger (`fori10`) sets the upper bound. Nesting a child with `>` fills the body.

### `hmap`

HashMap binding.

```sh
codemmet expand --lang rust "hmap"
```

```rust
let mut map = std::collections::HashMap::new();
```

Binds a new mutable `HashMap` to `map` using its fully qualified path.

### `if`

if block.

```sh
codemmet expand --lang rust "if"
```

```rust
if condition {
    
}
```

```sh
codemmet expand --lang rust "if>pr"
```

```rust
if condition {
    println!("");
}
```

An `if` block with a `condition` spot. Nesting a child with `>` fills the body.

### `ifl`

if let block.

```sh
codemmet expand --lang rust "ifl"
```

```rust
if let Some(value) = expr {
    
}
```

```sh
codemmet expand --lang rust "ifl>pr"
```

```rust
if let Some(value) = expr {
    println!("");
}
```

An `if let Some(value) = expr` block for pattern matching. Nesting a child with `>` fills the body.

### `impl`

inherent impl block.

```sh
codemmet expand --lang rust "impl:Point"
```

```rust
impl Point {
    
}
```

```sh
codemmet expand --lang rust "impl:Point>fn:new"
```

```rust
impl Point {
    fn new(&self) {
        
    }
}
```

An inherent `impl` block with a glued type argument (`:Point`). A `fn` nested with `>` becomes a method and gains a `&self` receiver.

### `implt`

trait impl block (impl Trait for Type).

```sh
codemmet expand --lang rust "implt"
```

```rust
impl Trait for Type {
    
}
```

```sh
codemmet expand --lang rust "implt>fn:fmt"
```

```rust
impl Trait for Type {
    fn fmt(&self) {
        
    }
}
```

An `impl Trait for Type` block; the `Trait` and `Type` are spots to fill. A `fn` nested with `>` becomes a method with `&self`.

### `let`

let binding.

```sh
codemmet expand --lang rust "let:count"
```

```rust
let count = ;
```

An immutable `let` binding with a glued name argument (`:count`) and an empty value spot.

### `letm`

let mut binding.

```sh
codemmet expand --lang rust "letm:count"
```

```rust
let mut count = ;
```

A mutable `let mut` binding with a glued name argument (`:count`) and an empty value spot.

### `letq`

let binding from a fallible expression with ?.

```sh
codemmet expand --lang rust "letq:file"
```

```rust
let file = ?;
```

A `let` binding whose value ends in the `?` operator, propagating errors from a fallible expression.

### `loop`

infinite loop.

```sh
codemmet expand --lang rust "loop"
```

```rust
loop {
    
}
```

```sh
codemmet expand --lang rust "loop>pr"
```

```rust
loop {
    println!("");
}
```

An infinite `loop` block. Nesting a child with `>` fills the body.

### `main`

main entry point.

```sh
codemmet expand --lang rust "main>pr"
```

```rust
fn main() {
    println!("");
}
```

The `fn main()` program entry point. Nesting a child with `>` fills the body.

### `match`

match expression.

```sh
codemmet expand --lang rust "match:cmd"
```

```rust
match cmd {
    
}
```

```sh
codemmet expand --lang rust "match:cmd>arm*2"
```

```rust
match cmd {
    pattern => ,
    pattern => ,
}
```

A `match` on a glued scrutinee argument (`:cmd`). Nest `arm` (optionally multiplied with `*n`) to add match arms.

### `matchres`

match on a Result with Ok/Err arms.

```sh
codemmet expand --lang rust "matchres"
```

```rust
match expr {
    Ok(value) => ,
    Err(err) => ,
}
```

```sh
codemmet expand --lang rust "rfn:load>matchres"
```

```rust
fn load() -> Result<(), Box<dyn std::error::Error>> {
    match expr {
        Ok(value) => ,
        Err(err) => ,
    }
}
```

A `match` on a `Result` with the `Ok` and `Err` arms already written. Nesting it inside a function body with `>` is a common pattern.

### `mod`

inline module.

```sh
codemmet expand --lang rust "mod:util"
```

```rust
mod util {
    
}
```

```sh
codemmet expand --lang rust "mod:util>fn:helper"
```

```rust
mod util {
    fn helper() {
        
    }
}
```

An inline `mod` with a glued name argument (`:util`). Nesting items with `>` places them inside the module.

### `pr`

println! to stdout.

```sh
codemmet expand --lang rust "pr"
```

```rust
println!("");
```

A `println!` writing to standard output, with an empty format string to fill.

### `prd`

debug-print a value with {:?}.

```sh
codemmet expand --lang rust "prd"
```

```rust
println!("{:?}", );
```

A `println!` using the `{:?}` debug formatter; the value to print goes in the trailing spot.

### `prf`

println! with a format string and arguments.

```sh
codemmet expand --lang rust "prf:\"x={}\",x"
```

```rust
println!("x={}", x);
```

A `println!` that takes explicit arguments: the format string and each value are passed after the colon, separated by commas.

### `ret`

return statement.

```sh
codemmet expand --lang rust "ret"
```

```rust
return ;
```

A `return` statement with an empty spot for the returned value.

### `rfn`

function returning Result.

```sh
codemmet expand --lang rust "rfn:load"
```

```rust
fn load() -> Result<(), Box<dyn std::error::Error>> {
    
}
```

```sh
codemmet expand --lang rust "rfn:load>ret"
```

```rust
fn load() -> Result<(), Box<dyn std::error::Error>> {
    return ;
}
```

A function with a glued name argument (`:load`) that returns a boxed-error `Result`. Nesting a child with `>` fills the body.

### `struct`

struct definition.

```sh
codemmet expand --lang rust "struct:Point"
```

```rust
struct Point {
    
}
```

```sh
codemmet expand --lang rust "struct:Point>doc"
```

```rust
struct Point {
    /// 
}
```

A `struct` with a glued name argument (`:Point`). Nesting with `>` places items such as a field doc comment inside the body.

### `test`

test function.

```sh
codemmet expand --lang rust "test:adds"
```

```rust
#[test]
fn adds() {
    
}
```

```sh
codemmet expand --lang rust "test:adds>aeq"
```

```rust
#[test]
fn adds() {
    assert_eq!(left, );
}
```

A `#[test]` function with a glued name argument (`:adds`). Nesting a child with `>`, such as `aeq`, fills the body.

### `tmod`

test module with cfg(test).

```sh
codemmet expand --lang rust "tmod"
```

```rust
#[cfg(test)]
mod tests {
    use super::*;

    
}
```

```sh
codemmet expand --lang rust "tmod>test:works"
```

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn works() {
        
    }
}
```

A `#[cfg(test)]` module with the conventional `use super::*;` import. Nest `test:<name>` functions inside with `>`.

### `todo`

TODO comment.

```sh
codemmet expand --lang rust "todo"
```

```rust
// TODO: 
```

A `// TODO:` comment line with the cursor left after the marker.

### `trait`

trait definition.

```sh
codemmet expand --lang rust "trait:Draw"
```

```rust
trait Draw {
    
}
```

```sh
codemmet expand --lang rust "trait:Draw>fn:draw"
```

```rust
trait Draw {
    fn draw(&self) {
        
    }
}
```

A `trait` with a glued name argument (`:Draw`). A `fn` nested with `>` becomes a trait method and gains a `&self` receiver.

### `use`

use declaration.

```sh
codemmet expand --lang rust "use:\"std::fmt\""
```

```rust
use std::fmt;
```

A `use` declaration; the path is passed as a glued argument.

### `vec`

vec! literal.

```sh
codemmet expand --lang rust "vec"
```

```rust
vec![]
```

A `vec![]` literal with an empty spot for the elements.

### `while`

while loop.

```sh
codemmet expand --lang rust "while"
```

```rust
while condition {
    
}
```

```sh
codemmet expand --lang rust "while>pr"
```

```rust
while condition {
    println!("");
}
```

A `while` loop with a `condition` spot. Nesting a child with `>` fills the body.

### `wl`

while let loop.

```sh
codemmet expand --lang rust "wl"
```

```rust
while let Some(value) = expr {
    
}
```

```sh
codemmet expand --lang rust "wl>pr"
```

```rust
while let Some(value) = expr {
    println!("");
}
```

A `while let Some(value) = expr` loop that runs as long as the pattern matches. Nesting a child with `>` fills the body.

## See also

- [abbreviation syntax](../syntax.md)
- [language reference](../languages.md)
- [examples overview](README.md)
