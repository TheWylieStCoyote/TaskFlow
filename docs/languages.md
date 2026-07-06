# Language reference

codemmet ships with **snippet packs** for seven languages: Rust, Python, C,
C++, JavaScript, TypeScript, and Go. Each pack registers a set of **triggers**
tuned to that language's idioms.

You select a pack with the `--lang` flag. Matching is flexible: `--lang`
accepts the language **id** (such as `rust`), any of the language's file
**extensions** (such as `rs`), or any of its **LSP language ids** (such as
`rust`). All three of these select the Rust pack:

```sh
codemmet expand --lang rust "fn:main"
codemmet expand --lang rs   "fn:main"
```

Every trigger declares parameters. Any parameter you do not fill with a
[glued or explicit argument](syntax.md) becomes a **tabstop** in the
expansion. Two of the packs **inherit** another: C++ starts from the C pack and
overrides or adds triggers on top of it, and TypeScript starts from the
JavaScript pack.

You can regenerate any of the trigger lists below live from your installed
build with `codemmet list --lang <id>`, and `--json` is available for
machine-readable output:

```sh
codemmet list --lang rust
codemmet list --lang rust --json
```

## rust

id `rust` | extensions: `rs` | LSP ids: `rust` | default indent: 4 spaces

| Trigger | Description |
| --- | --- |
| `aeq` | assert_eq! statement |
| `afn` | async function definition |
| `arm` | match arm |
| `cl` | closure |
| `const` | const item |
| `derive` | derive attribute |
| `doc` | doc comment line |
| `el` | else block |
| `elif` | else-if block |
| `enum` | enum definition |
| `epr` | eprintln! to stderr |
| `fn` | function definition |
| `fore` | for-each loop over an iterator |
| `fori` | for loop over integer range |
| `hmap` | HashMap binding |
| `if` | if block |
| `ifl` | if let block |
| `impl` | inherent impl block |
| `implt` | trait impl block (impl Trait for Type) |
| `let` | let binding |
| `letm` | let mut binding |
| `letq` | let binding from a fallible expression with ? |
| `loop` | infinite loop |
| `main` | main entry point |
| `match` | match expression |
| `matchres` | match on a Result with Ok/Err arms |
| `mod` | inline module |
| `pr` | println! to stdout |
| `prd` | debug-print a value with {:?} |
| `prf` | println! with a format string and arguments |
| `ret` | return statement |
| `rfn` | function returning Result |
| `struct` | struct definition |
| `test` | test function |
| `tmod` | test module with cfg(test) |
| `todo` | TODO comment |
| `trait` | trait definition |
| `use` | use declaration |
| `vec` | vec! literal |
| `while` | while loop |
| `wl` | while let loop |

## python

id `python` | extensions: `py` | LSP ids: `python` | default indent: 4 spaces

| Trigger | Description |
| --- | --- |
| `adef` | async function definition |
| `aeq` | equality assertion |
| `arm` | case clause of a match statement |
| `class` | class definition |
| `dc` | dict comprehension |
| `def` | function definition |
| `doc` | docstring |
| `el` | else block |
| `elif` | elif block |
| `exc` | except clause |
| `fin` | finally clause |
| `fore` | for-each loop over an iterable |
| `fori` | for loop over integer range |
| `from` | from-import statement |
| `if` | if block |
| `ifmain` | __main__ guard block |
| `imp` | import statement |
| `init` | __init__ method |
| `lam` | lambda expression |
| `lc` | list comprehension |
| `main` | main function with __main__ guard |
| `match` | match statement |
| `pr` | print call |
| `prd` | debug print with f-string = specifier |
| `prop` | property getter |
| `raise` | raise an exception |
| `repr` | __repr__ dunder method |
| `ret` | return statement |
| `str` | __str__ dunder method |
| `test` | test function (pytest style) |
| `todo` | TODO comment |
| `try` | try/except block |
| `while` | while loop |
| `with` | with statement (context manager) |

Nesting a class body illustrates how unfilled parameters become tabstops:

```sh
codemmet expand --lang python "class:Point>init+str"
```

```python
class Point:
    def __init__(self):
        
    def __str__(self) -> str:
        return f""
```

## c

id `c` | extensions: `c`, `h` | LSP ids: `c` | default indent: 4 spaces

| Trigger | Description |
| --- | --- |
| `aeq` | equality assertion |
| `assert` | assert statement |
| `case` | case label with break |
| `def` | #define constant |
| `default` | default label with break |
| `doc` | doc comment line |
| `dowhile` | do-while loop |
| `el` | else block |
| `elif` | else-if block |
| `enum` | typedef'd enum definition |
| `fn` | function definition |
| `fori` | for loop over integer range |
| `free` | free a pointer and null it |
| `guard` | include guard |
| `if` | if block |
| `inc` | #include a system header |
| `incl` | #include a local header |
| `main` | main entry point |
| `malloc` | malloc a typed buffer |
| `pr` | printf a line to stdout |
| `prf` | printf with a format string and arguments |
| `ret` | return statement |
| `sizeof` | sizeof expression |
| `struct` | typedef'd struct definition |
| `switch` | switch statement |
| `td` | typedef declaration |
| `todo` | TODO comment |
| `while` | while loop |

## cpp

id `cpp` | extensions: `cpp`, `cc`, `cxx`, `hpp`, `hh`, `hxx` | LSP ids: `cpp` |
default indent: 4 spaces | inherits: the C pack

C++ starts from the C pack and overrides or adds triggers on top of it. For
example, `enum` and `struct` override C's typedef forms.

| Trigger | Description |
| --- | --- |
| `aeq` | equality assertion |
| `assert` | assert statement |
| `case` | case label with break |
| `class` | class definition |
| `cout` | std::cout line |
| `def` | #define constant |
| `default` | default label with break |
| `doc` | doc comment line |
| `dowhile` | do-while loop |
| `el` | else block |
| `elif` | else-if block |
| `enum` | scoped enum (overrides C's typedef form) |
| `fn` | function definition |
| `fore` | range-based for loop |
| `fori` | for loop over integer range |
| `free` | free a pointer and null it |
| `guard` | include guard |
| `if` | if block |
| `inc` | #include a system header |
| `incl` | #include a local header |
| `lam` | lambda expression |
| `main` | main entry point |
| `malloc` | malloc a typed buffer |
| `map` | std::unordered_map variable |
| `ns` | namespace block |
| `pr` | printf a line to stdout |
| `prf` | printf with a format string and arguments |
| `ret` | return statement |
| `sizeof` | sizeof expression |
| `sptr` | std::make_shared binding |
| `struct` | struct definition (overrides C's typedef form) |
| `switch` | switch statement |
| `td` | typedef declaration |
| `todo` | TODO comment |
| `tpl` | template parameter list (prefix for a declaration) |
| `try` | try/catch block |
| `uptr` | std::make_unique binding |
| `using` | using type alias |
| `vec` | std::vector variable |
| `while` | while loop |

## javascript

id `javascript` | extensions: `js`, `mjs`, `cjs`, `jsx` |
LSP ids: `javascript`, `javascriptreact` | default indent: 2 spaces

| Trigger | Description |
| --- | --- |
| `aeq` | equality expectation |
| `afn` | async function declaration |
| `arrow` | arrow function bound to a const |
| `aw` | await statement |
| `case` | case clause with break |
| `class` | class definition |
| `ctor` | constructor method |
| `default` | default clause |
| `doc` | JSDoc comment line |
| `el` | else block |
| `elif` | else-if block |
| `exp` | export a const binding |
| `expd` | export default |
| `fn` | function declaration |
| `fore` | for...of loop |
| `fori` | for loop over integer range |
| `forin` | for...in loop over object keys |
| `get` | getter backed by a private-by-convention field |
| `if` | if block |
| `imp` | default import |
| `impd` | destructured (named) import |
| `jsonp` | JSON.parse expression |
| `jsons` | JSON.stringify expression |
| `pr` | console.log call |
| `prd` | debug-log with object-shorthand label |
| `prom` | new Promise with executor |
| `ret` | return statement |
| `set` | setter backed by a private-by-convention field |
| `switch` | switch statement |
| `test` | test case (jest/vitest style) |
| `todo` | TODO comment |
| `try` | try/catch block |
| `while` | while loop |

## typescript

id `typescript` | extensions: `ts`, `mts`, `cts`, `tsx` |
LSP ids: `typescript`, `typescriptreact` | default indent: 2 spaces |
inherits: the JavaScript pack

TypeScript starts from the JavaScript pack. Several triggers such as `afn` and
`fn` override the JavaScript versions to emit typed forms.

| Trigger | Description |
| --- | --- |
| `aeq` | equality expectation |
| `afn` | typed async function declaration (overrides JS) |
| `arrow` | arrow function bound to a const |
| `asrt` | as-type assertion |
| `aw` | await statement |
| `case` | case clause with break |
| `class` | class definition |
| `ctor` | constructor method |
| `default` | default clause |
| `doc` | JSDoc comment line |
| `el` | else block |
| `elif` | else-if block |
| `enum` | enum definition |
| `exp` | export a const binding |
| `expd` | export default |
| `fn` | typed function declaration (overrides JS) |
| `fng` | generic function declaration |
| `fore` | for...of loop |
| `fori` | for loop over integer range |
| `forin` | for...in loop over object keys |
| `get` | getter backed by a private-by-convention field |
| `if` | if block |
| `imp` | default import |
| `impd` | destructured (named) import |
| `impt` | type-only import |
| `int` | interface definition |
| `jsonp` | JSON.parse expression |
| `jsons` | JSON.stringify expression |
| `pr` | console.log call |
| `prd` | debug-log with object-shorthand label |
| `prom` | new Promise with executor |
| `ret` | return statement |
| `set` | setter backed by a private-by-convention field |
| `switch` | switch statement |
| `test` | test case (jest/vitest style) |
| `todo` | TODO comment |
| `try` | try/catch block |
| `typ` | type alias |
| `while` | while loop |

## go

id `go` | extensions: `go` | LSP ids: `go` | default indent: tab

| Trigger | Description |
| --- | --- |
| `aeq` | equality check reporting through t.Errorf |
| `case` | case clause |
| `chan` | channel binding |
| `const` | const declaration |
| `default` | default clause |
| `defer` | defer statement |
| `doc` | doc comment line |
| `el` | else block (join onto the previous `}` by hand) |
| `elif` | else-if block (join onto the previous `}` by hand) |
| `err` | errors.New expression |
| `fn` | function definition |
| `fnr` | function with return type |
| `for` | bare (infinite) for loop |
| `forc` | for loop with condition (Go's while) |
| `fore` | range loop over values |
| `fori` | for loop over integer range |
| `forr` | range loop with index and value |
| `go` | goroutine over an anonymous function |
| `if` | if block |
| `iface` | interface type definition |
| `iferr` | if err != nil guard returning the error |
| `imp` | import block |
| `let` | short variable declaration (:=) |
| `main` | main entry point |
| `map` | map binding |
| `meth` | method with receiver |
| `pkg` | package clause |
| `pr` | fmt.Println call |
| `prd` | debug-print a value with %+v |
| `prf` | fmt.Printf with a format string and arguments |
| `ret` | return statement |
| `sel` | select statement (cases at select level, per gofmt) |
| `struct` | struct type definition |
| `switch` | switch statement (cases at switch level, per gofmt) |
| `test` | test function (testing package) |
| `todo` | TODO comment |

Go uses tab indentation, which you can see in this expansion:

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

A couple of Go caveats are worth knowing. The `el` and `elif` triggers land on
their own line, so you join them onto the previous `}` by hand. And per gofmt,
the cases produced by `switch` and `sel` sit at the switch level rather than
indented under it.

## See also

- [documentation home](README.md)
- [abbreviation syntax](syntax.md)
- [CLI reference](cli.md)
- [custom snippets](snippets.md)
- [editor setup](editors.md)
- [tutorials](tutorials.md)
- [troubleshooting](troubleshooting.md)
