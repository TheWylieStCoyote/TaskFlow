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

## See also

- [abbreviation syntax](../syntax.md)
- [language reference](../languages.md)
- [examples overview](README.md)
