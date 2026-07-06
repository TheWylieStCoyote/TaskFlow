# Go examples

The `go` language (id `go`, extension `go`) is matched by `--lang` through its id, its `go` extension, or its LSP id, and it indents with a TAB by default to match gofmt. See the [language reference](../languages.md) for the full trigger list.

> Examples use the default `strip` mode, which renders plain code and leaves an empty spot wherever a tabstop will be — those are the places you fill in. Add `--tabstops keep` to see them as `$1` / `${1:placeholder}` / `$0` markers. See the [examples overview](README.md) for more.

Note that Go's `el`/`elif` triggers render the keyword on its own line, so you join them onto the previous `}` by hand — this is inherent to line-wise composition.

## A function with an error guard

```sh
codemmet expand --lang go "fn:run>let:cfg+iferr+ret"
```

```go
func run() {
	cfg := 
	if err != nil {
		return err
	}
	
	return 
}
```

The `>` nests three siblings inside the function body, and the `+` joins them in order: a short declaration (`cfg :=`), the classic `if err != nil { return err }` guard, and a final `return`. Fill each empty spot to complete the body.

## A struct type

```sh
codemmet expand --lang go "struct:Server"
```

```go
type Server struct {
	
}
```

The `struct` trigger takes `Server` as a glued argument, which becomes the type name. The body is left open for you to add fields.

## A method with a receiver

```sh
codemmet expand --lang go "meth"
```

```go
func (r *Type) name() {
	
}
```

Go methods are top-level declarations with a receiver rather than members nested inside the struct, so `meth` expands on its own. Replace the receiver, `Type`, and `name` tabstops with your own values.

## Ranging over a slice

```sh
codemmet expand --lang go "fn:sum>forr>pr"
```

```go
func sum() {
	for i, v := range items {
		fmt.Println()
	}
}
```

`forr` is the index-and-value range form (`for i, v := range items`), nested inside the function with `>`, and `pr` drops a `fmt.Println()` call inside the loop.

## A goroutine over a channel

```sh
codemmet expand --lang go "fn:worker>chan+go"
```

```go
func worker() {
	ch := make(chan int)
	go func() {
		
	}()
}
```

The two siblings joined with `+` set up the channel (`ch := make(chan int)`) and then launch an anonymous goroutine with `go func() { … }()`.

## A select loop

```sh
codemmet expand --lang go "for>sel>case+default"
```

```go
for {
	select {
	case value:
		
	default:
		
	}
}
```

A bare `for` wraps a `select`, and the `case` and `default` siblings sit at the select's level per gofmt. Fill in the case expression and each branch body.

## An interface

```sh
codemmet expand --lang go "iface:Reader"
```

```go
type Reader interface {
	
}
```

The `iface` trigger takes `Reader` as a glued argument for the interface name and leaves the method set for you to fill in.

## A table-free test

```sh
codemmet expand --lang go "test:Add>aeq"
```

```go
func TestAdd(t *testing.T) {
	if got != want {
		t.Errorf("got %v, want %v", got, want)
	}
	
}
```

The `test` trigger builds the standard `testing`-package function `TestAdd(t *testing.T)`, and `aeq` nests an equality assertion that reports through `t.Errorf`.

## Trigger reference

Every Go trigger, alphabetically. Block and named triggers show a simple form and a composed form that nests a child; leaf triggers show a single form.

### `aeq`

Equality check that reports a mismatch through `t.Errorf`.

```sh
codemmet expand --lang go "aeq"
```

```go
if got != want {
	t.Errorf("got %v, want %v", got, want)
}

```

`aeq` drops the standard `if got != want { … }` assertion used in table-free tests. Replace `got` and `want` with the values you are comparing.

### `case`

A `case` clause for a `switch` or `select`.

```sh
codemmet expand --lang go "case"
```

```go
case value:
	
```

```sh
codemmet expand --lang go "switch:status>case+default"
```

```go
switch status {
case value:
	
default:
	
}
```

On its own `case` emits a single clause with its body indented one level. Nested under `switch:status` with a `+default` sibling, the clauses sit at the switch's level per gofmt.

### `chan`

Channel binding with `make`.

```sh
codemmet expand --lang go "chan"
```

```go
ch := make(chan int)
```

`chan` declares a channel via `ch := make(chan int)`. Change the name and element type to suit.

### `const`

Const declaration.

```sh
codemmet expand --lang go "const"
```

```go
const name = 
```

`const` emits a single `const name = ` binding, leaving the name and value as tabstops.

### `default`

A `default` clause for a `switch` or `select`.

```sh
codemmet expand --lang go "default"
```

```go
default:
	
```

```sh
codemmet expand --lang go "sel>case+default"
```

```go
select {
case value:
	
default:
	
}
```

Alone, `default` emits the clause with an indented body. Nested inside a `sel` with a `case` sibling, both clauses sit at the select's level per gofmt.

### `defer`

Defer statement.

```sh
codemmet expand --lang go "defer"
```

```go
defer 
```

`defer` emits the `defer` keyword and leaves the deferred call as a tabstop.

### `doc`

Doc comment line.

```sh
codemmet expand --lang go "doc"
```

```go
// 
```

`doc` inserts a single `// ` comment line for documenting the declaration that follows.

### `el`

Else block. It renders the keyword on its own line, so join it onto the previous `}` by hand.

```sh
codemmet expand --lang go "el"
```

```go
else {
	
}
```

```sh
codemmet expand --lang go "if+el"
```

```go
if condition {
	
}
else {
	
}
```

Composed as `if+el`, the `else` lands on the line after the `if`'s closing brace; join it onto that `}` yourself, since line-wise composition can't do it for you.

### `elif`

Else-if block. Like `el`, the keyword renders on its own line, so join it onto the previous `}` by hand.

```sh
codemmet expand --lang go "elif"
```

```go
else if condition {
	
}
```

```sh
codemmet expand --lang go "if+elif"
```

```go
if condition {
	
}
else if condition {
	
}
```

Composed as `if+elif`, the `else if` follows the `if`'s closing brace on its own line; move it up onto that `}` by hand.

### `err`

`errors.New` expression.

```sh
codemmet expand --lang go "err"
```

```go
errors.New("")
```

`err` produces an `errors.New("")` call with the message string left empty for you to fill.

### `fn`

Function definition.

```sh
codemmet expand --lang go "fn:run"
```

```go
func run() {
	
}
```

```sh
codemmet expand --lang go "fn:run>iferr"
```

```go
func run() {
	if err != nil {
		return err
	}
	
}
```

`fn` takes the function name as a glued argument (`fn:run`). The `>` nests children into the body — here an `iferr` guard.

### `fnr`

Function with an `error` return type.

```sh
codemmet expand --lang go "fnr:parse"
```

```go
func parse() error {
	
}
```

```sh
codemmet expand --lang go "fnr:parse>iferr+ret"
```

```go
func parse() error {
	if err != nil {
		return err
	}
	
	return 
}
```

`fnr:parse` builds `func parse() error`. The composed form nests an `iferr` guard and a trailing `return`, joined with `+`.

### `for`

Bare (infinite) for loop.

```sh
codemmet expand --lang go "for"
```

```go
for {
	
}
```

```sh
codemmet expand --lang go "for>sel>case+default"
```

```go
for {
	select {
	case value:
		
	default:
		
	}
}
```

`for` alone is Go's infinite loop. The composed form wraps a `select` whose `case` and `default` clauses sit at the select's level per gofmt.

### `forc`

For loop with a condition (Go's `while`).

```sh
codemmet expand --lang go "forc"
```

```go
for condition {
	
}
```

```sh
codemmet expand --lang go "forc>pr"
```

```go
for condition {
	fmt.Println()
}
```

`forc` is the single-condition loop that stands in for `while`. Replace `condition`, and nest the body with `>`.

### `fore`

Range loop over values.

```sh
codemmet expand --lang go "fore"
```

```go
for _, item := range items {
	
}
```

```sh
codemmet expand --lang go "fore>pr"
```

```go
for _, item := range items {
	fmt.Println()
}
```

`fore` ranges over values while discarding the index (`for _, item := range items`). The `>pr` nests a `fmt.Println()` into the body.

### `fori`

For loop over an integer range.

```sh
codemmet expand --lang go "fori10"
```

```go
for i := 0; i < 10; i++ {
	
}
```

```sh
codemmet expand --lang go "fori10>pr"
```

```go
for i := 0; i < 10; i++ {
	fmt.Println()
}
```

The glued argument sets the bound, so `fori10` counts `i` from 0 to 10. Nest the body with `>`.

### `forr`

Range loop with both index and value.

```sh
codemmet expand --lang go "forr"
```

```go
for i, v := range items {
	
}
```

```sh
codemmet expand --lang go "fn:sum>forr>pr"
```

```go
func sum() {
	for i, v := range items {
		fmt.Println()
	}
}
```

`forr` binds both `i` and `v` (`for i, v := range items`). Here it is nested inside a function with a `fmt.Println()` in its body.

### `go`

Goroutine over an anonymous function.

```sh
codemmet expand --lang go "go"
```

```go
go func() {
	
}()
```

```sh
codemmet expand --lang go "fn:worker>chan+go"
```

```go
func worker() {
	ch := make(chan int)
	go func() {
		
	}()
}
```

`go` launches an anonymous `go func() { … }()`. The composed form sets up a channel and then the goroutine as `+`-joined siblings.

### `if`

If block.

```sh
codemmet expand --lang go "if"
```

```go
if condition {
	
}
```

```sh
codemmet expand --lang go "if>ret"
```

```go
if condition {
	return 
}
```

`if` emits the condition and an open body. Nest statements with `>`, such as the `return` here.

### `iface`

Interface type definition.

```sh
codemmet expand --lang go "iface:Reader"
```

```go
type Reader interface {
	
}
```

```sh
codemmet expand --lang go "iface:Reader>doc"
```

```go
type Reader interface {
	// 
}
```

`iface:Reader` names the interface via a glued argument. Nest the method set (or a `doc` comment) with `>`.

### `iferr`

The `if err != nil` guard that returns the error.

```sh
codemmet expand --lang go "iferr"
```

```go
if err != nil {
	return err
}

```

```sh
codemmet expand --lang go "fn:run>iferr"
```

```go
func run() {
	if err != nil {
		return err
	}
	
}
```

`iferr` is Go's ubiquitous error check, emitting `if err != nil { return err }`. Here it is nested into a function body.

### `imp`

Import block.

```sh
codemmet expand --lang go "imp"
```

```go
import (
	"fmt"
)
```

`imp` opens a parenthesized `import` block seeded with `"fmt"`. Add or replace the import paths inside.

### `let`

Short variable declaration (`:=`).

```sh
codemmet expand --lang go "let"
```

```go
name := 
```

`let` emits a `name := ` short declaration, leaving the name and value as tabstops.

### `main`

The `main` entry point.

```sh
codemmet expand --lang go "main"
```

```go
func main() {
	
}
```

```sh
codemmet expand --lang go "main>pr"
```

```go
func main() {
	fmt.Println()
}
```

`main` builds `func main()`. The composed form nests a `fmt.Println()` into the body with `>`.

### `map`

Map binding with `make`.

```sh
codemmet expand --lang go "map"
```

```go
m := make(map[string]int)
```

`map` declares a map via `m := make(map[string]int)`. Adjust the key and value types as needed.

### `meth`

Method with a receiver.

```sh
codemmet expand --lang go "meth"
```

```go
func (r *Type) name() {
	
}
```

```sh
codemmet expand --lang go "meth>iferr"
```

```go
func (r *Type) name() {
	if err != nil {
		return err
	}
	
}
```

Go methods are top-level declarations with a receiver, so `meth` expands on its own rather than inside a struct. Replace the receiver, `Type`, and `name`, and nest body statements with `>`.

### `pkg`

Package clause.

```sh
codemmet expand --lang go "pkg"
```

```go
package main
```

`pkg` emits the `package main` clause at the top of a file. Rename the package as appropriate.

### `pr`

`fmt.Println` call.

```sh
codemmet expand --lang go "pr"
```

```go
fmt.Println()
```

`pr` drops a bare `fmt.Println()` call; add the arguments to print inside the parentheses.

### `prd`

Debug-print a value with `%+v`.

```sh
codemmet expand --lang go "prd"
```

```go
fmt.Printf("%+v\n", )
```

`prd` is a debugging print that formats a value with `%+v` to show field names, leaving the argument as a tabstop.

### `prf`

`fmt.Printf` with a format string and arguments.

```sh
codemmet expand --lang go "prf"
```

```go
fmt.Printf("%v\n", value)
```

`prf` emits a `fmt.Printf` call with a `%v` format string and a `value` argument to customize.

### `ret`

Return statement.

```sh
codemmet expand --lang go "ret"
```

```go
return 
```

`ret` emits the `return` keyword and leaves the returned expression as a tabstop.

### `sel`

Select statement. Its cases sit at the select level per gofmt.

```sh
codemmet expand --lang go "sel"
```

```go
select {

}
```

```sh
codemmet expand --lang go "sel>case+default"
```

```go
select {
case value:
	
default:
	
}
```

`sel` opens a `select` block. Nested `case` and `default` clauses sit at the select's own level, not indented further, matching gofmt.

### `struct`

Struct type definition.

```sh
codemmet expand --lang go "struct:Server"
```

```go
type Server struct {
	
}
```

```sh
codemmet expand --lang go "struct:Server>doc"
```

```go
type Server struct {
	// 
}
```

`struct:Server` names the type via a glued argument and leaves the body open for fields. Nest a `doc` comment or fields with `>`.

### `switch`

Switch statement. Its cases sit at the switch level per gofmt.

```sh
codemmet expand --lang go "switch"
```

```go
switch value {

}
```

```sh
codemmet expand --lang go "switch:status>case+default"
```

```go
switch status {
case value:
	
default:
	
}
```

`switch` alone emits `switch value { }`; the glued argument (`switch:status`) sets the switched expression. Nested `case`/`default` clauses sit at the switch's own level per gofmt.

### `test`

Test function using the `testing` package.

```sh
codemmet expand --lang go "test:Add"
```

```go
func TestAdd(t *testing.T) {
	
}
```

```sh
codemmet expand --lang go "test:Add>aeq"
```

```go
func TestAdd(t *testing.T) {
	if got != want {
		t.Errorf("got %v, want %v", got, want)
	}
	
}
```

`test:Add` builds the standard `func TestAdd(t *testing.T)`. The composed form nests an `aeq` equality assertion into the body.

### `todo`

TODO comment.

```sh
codemmet expand --lang go "todo"
```

```go
// TODO: 
```

`todo` inserts a `// TODO: ` marker for a note you intend to come back to.

## See also

- [abbreviation syntax](../syntax.md)
- [language reference](../languages.md)
- [examples overview](README.md)
