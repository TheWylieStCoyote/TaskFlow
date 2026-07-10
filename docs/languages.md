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

Every trigger name in the tables below links to a detailed, runnable example
on that language's page under [examples](examples/README.md).

The tables list each pack's base triggers. Every pack also ships a few
**composite** triggers — aliases that expand to a scaffold built from these
base triggers (Rust `dstruct`/`denum`, Go `stringer`/`errtype`, C `hstruct`/
`henum`, C++ `hclass`/`nsclass`, Python `classp`/`ctxmgr`, and more). They show
up in `codemmet list` alongside the base triggers and are documented, with the
full built-in set, under
[custom snippets → composite snippets](snippets.md#composite-snippets).

A few triggers are **multi-file** snippets — they scaffold across several files
and are run with [`codemmet scaffold`](cli.md#scaffold) rather than `expand`
(module/package scaffolds `mmod`/`mpkg`, test files `itest`, Rust `mbin`, and
C++ `mmethod`/`mns`/`mcls`). See
[custom snippets → multi-file snippets](snippets.md#multi-file-snippets).

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
| [`aeq`](examples/rust.md#aeq) | assert_eq! statement |
| [`afn`](examples/rust.md#afn) | async function definition |
| [`arm`](examples/rust.md#arm) | match arm |
| [`cl`](examples/rust.md#cl) | closure |
| [`const`](examples/rust.md#const) | const item |
| [`derive`](examples/rust.md#derive) | derive attribute |
| [`doc`](examples/rust.md#doc) | doc comment line |
| [`el`](examples/rust.md#el) | else block |
| [`elif`](examples/rust.md#elif) | else-if block |
| [`enum`](examples/rust.md#enum) | enum definition |
| [`epr`](examples/rust.md#epr) | eprintln! to stderr |
| [`fn`](examples/rust.md#fn) | function definition |
| [`fore`](examples/rust.md#fore) | for-each loop over an iterator |
| [`fori`](examples/rust.md#fori) | for loop over integer range |
| [`hmap`](examples/rust.md#hmap) | HashMap binding |
| [`if`](examples/rust.md#if) | if block |
| [`ifl`](examples/rust.md#ifl) | if let block |
| [`impl`](examples/rust.md#impl) | inherent impl block |
| [`implt`](examples/rust.md#implt) | trait impl block (impl Trait for Type) |
| [`let`](examples/rust.md#let) | let binding |
| [`letm`](examples/rust.md#letm) | let mut binding |
| [`letq`](examples/rust.md#letq) | let binding from a fallible expression with ? |
| [`loop`](examples/rust.md#loop) | infinite loop |
| [`main`](examples/rust.md#main) | main entry point |
| [`match`](examples/rust.md#match) | match expression |
| [`matchres`](examples/rust.md#matchres) | match on a Result with Ok/Err arms |
| [`mod`](examples/rust.md#mod) | inline module |
| [`mtest`](examples/rust.md#mtest) | grouped test case, named `${outer}_${value}` |
| [`pr`](examples/rust.md#pr) | println! to stdout |
| [`prd`](examples/rust.md#prd) | debug-print a value with {:?} |
| [`prf`](examples/rust.md#prf) | println! with a format string and arguments |
| [`ret`](examples/rust.md#ret) | return statement |
| [`rfn`](examples/rust.md#rfn) | function returning Result |
| [`struct`](examples/rust.md#struct) | struct definition |
| [`test`](examples/rust.md#test) | test function |
| [`tmod`](examples/rust.md#tmod) | test module with cfg(test) |
| [`todo`](examples/rust.md#todo) | TODO comment |
| [`trait`](examples/rust.md#trait) | trait definition |
| [`use`](examples/rust.md#use) | use declaration |
| [`vec`](examples/rust.md#vec) | vec! literal |
| [`while`](examples/rust.md#while) | while loop |
| [`wl`](examples/rust.md#wl) | while let loop |

## python

id `python` | extensions: `py` | LSP ids: `python` | default indent: 4 spaces

| Trigger | Description |
| --- | --- |
| [`adef`](examples/python.md#adef) | async function definition |
| [`aeq`](examples/python.md#aeq) | equality assertion |
| [`arm`](examples/python.md#arm) | case clause of a match statement |
| [`class`](examples/python.md#class) | class definition |
| [`dc`](examples/python.md#dc) | dict comprehension |
| [`def`](examples/python.md#def) | function definition |
| [`doc`](examples/python.md#doc) | docstring |
| [`el`](examples/python.md#el) | else block |
| [`elif`](examples/python.md#elif) | elif block |
| [`exc`](examples/python.md#exc) | except clause |
| [`fin`](examples/python.md#fin) | finally clause |
| [`fore`](examples/python.md#fore) | for-each loop over an iterable |
| [`fori`](examples/python.md#fori) | for loop over integer range |
| [`from`](examples/python.md#from) | from-import statement |
| [`if`](examples/python.md#if) | if block |
| [`ifmain`](examples/python.md#ifmain) | __main__ guard block |
| [`imp`](examples/python.md#imp) | import statement |
| [`init`](examples/python.md#init) | __init__ method |
| [`lam`](examples/python.md#lam) | lambda expression |
| [`lc`](examples/python.md#lc) | list comprehension |
| [`main`](examples/python.md#main) | main function with __main__ guard |
| [`match`](examples/python.md#match) | match statement |
| [`pr`](examples/python.md#pr) | print call |
| [`prd`](examples/python.md#prd) | debug print with f-string = specifier |
| [`prop`](examples/python.md#prop) | property getter |
| [`raise`](examples/python.md#raise) | raise an exception |
| [`repr`](examples/python.md#repr) | __repr__ dunder method |
| [`ret`](examples/python.md#ret) | return statement |
| [`str`](examples/python.md#str) | __str__ dunder method |
| [`test`](examples/python.md#test) | test function (pytest style) |
| [`todo`](examples/python.md#todo) | TODO comment |
| [`try`](examples/python.md#try) | try/except block |
| [`while`](examples/python.md#while) | while loop |
| [`with`](examples/python.md#with) | with statement (context manager) |

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
| [`aeq`](examples/c.md#aeq) | equality assertion |
| [`assert`](examples/c.md#assert) | assert statement |
| [`case`](examples/c.md#case) | case label with break |
| [`def`](examples/c.md#def) | #define constant |
| [`default`](examples/c.md#default) | default label with break |
| [`doc`](examples/c.md#doc) | doc comment line |
| [`dowhile`](examples/c.md#dowhile) | do-while loop |
| [`el`](examples/c.md#el) | else block |
| [`elif`](examples/c.md#elif) | else-if block |
| [`enum`](examples/c.md#enum) | typedef'd enum definition |
| [`fn`](examples/c.md#fn) | function definition |
| [`fori`](examples/c.md#fori) | for loop over integer range |
| [`free`](examples/c.md#free) | free a pointer and null it |
| [`guard`](examples/c.md#guard) | include guard |
| [`if`](examples/c.md#if) | if block |
| [`inc`](examples/c.md#inc) | #include a system header |
| [`incl`](examples/c.md#incl) | #include a local header |
| [`main`](examples/c.md#main) | main entry point |
| [`malloc`](examples/c.md#malloc) | malloc a typed buffer |
| [`pr`](examples/c.md#pr) | printf a line to stdout |
| [`prf`](examples/c.md#prf) | printf with a format string and arguments |
| [`ret`](examples/c.md#ret) | return statement |
| [`sizeof`](examples/c.md#sizeof) | sizeof expression |
| [`struct`](examples/c.md#struct) | typedef'd struct definition |
| [`switch`](examples/c.md#switch) | switch statement |
| [`td`](examples/c.md#td) | typedef declaration |
| [`todo`](examples/c.md#todo) | TODO comment |
| [`while`](examples/c.md#while) | while loop |

## cpp

id `cpp` | extensions: `cpp`, `cc`, `cxx`, `hpp`, `hh`, `hxx` | LSP ids: `cpp` |
default indent: 4 spaces | inherits: the C pack

C++ starts from the C pack and overrides or adds triggers on top of it. For
example, `enum` and `struct` override C's typedef forms.

| Trigger | Description |
| --- | --- |
| [`aeq`](examples/cpp.md#aeq) | equality assertion |
| [`assert`](examples/cpp.md#assert) | assert statement |
| [`case`](examples/cpp.md#case) | case label with break |
| [`class`](examples/cpp.md#class) | class definition |
| [`cout`](examples/cpp.md#cout) | std::cout line |
| [`def`](examples/cpp.md#def) | #define constant |
| [`default`](examples/cpp.md#default) | default label with break |
| [`doc`](examples/cpp.md#doc) | doc comment line |
| [`dowhile`](examples/cpp.md#dowhile) | do-while loop |
| [`el`](examples/cpp.md#el) | else block |
| [`elif`](examples/cpp.md#elif) | else-if block |
| [`enum`](examples/cpp.md#enum) | scoped enum (overrides C's typedef form) |
| [`fn`](examples/cpp.md#fn) | function definition |
| [`fore`](examples/cpp.md#fore) | range-based for loop |
| [`fori`](examples/cpp.md#fori) | for loop over integer range |
| [`free`](examples/cpp.md#free) | free a pointer and null it |
| [`guard`](examples/cpp.md#guard) | include guard |
| [`if`](examples/cpp.md#if) | if block |
| [`inc`](examples/cpp.md#inc) | #include a system header |
| [`incl`](examples/cpp.md#incl) | #include a local header |
| [`lam`](examples/cpp.md#lam) | lambda expression |
| [`main`](examples/cpp.md#main) | main entry point |
| [`malloc`](examples/cpp.md#malloc) | malloc a typed buffer |
| [`map`](examples/cpp.md#map) | std::unordered_map variable |
| [`ns`](examples/cpp.md#ns) | namespace block |
| [`pr`](examples/cpp.md#pr) | printf a line to stdout |
| [`prf`](examples/cpp.md#prf) | printf with a format string and arguments |
| [`ret`](examples/cpp.md#ret) | return statement |
| [`sizeof`](examples/cpp.md#sizeof) | sizeof expression |
| [`sptr`](examples/cpp.md#sptr) | std::make_shared binding |
| [`struct`](examples/cpp.md#struct) | struct definition (overrides C's typedef form) |
| [`switch`](examples/cpp.md#switch) | switch statement |
| [`td`](examples/cpp.md#td) | typedef declaration |
| [`todo`](examples/cpp.md#todo) | TODO comment |
| [`tpl`](examples/cpp.md#tpl) | template parameter list (prefix for a declaration) |
| [`try`](examples/cpp.md#try) | try/catch block |
| [`uptr`](examples/cpp.md#uptr) | std::make_unique binding |
| [`using`](examples/cpp.md#using) | using type alias |
| [`vec`](examples/cpp.md#vec) | std::vector variable |
| [`while`](examples/cpp.md#while) | while loop |

## javascript

id `javascript` | extensions: `js`, `mjs`, `cjs`, `jsx` |
LSP ids: `javascript`, `javascriptreact` | default indent: 2 spaces

| Trigger | Description |
| --- | --- |
| [`aeq`](examples/javascript.md#aeq) | equality expectation |
| [`afn`](examples/javascript.md#afn) | async function declaration |
| [`arrow`](examples/javascript.md#arrow) | arrow function bound to a const |
| [`aw`](examples/javascript.md#aw) | await statement |
| [`case`](examples/javascript.md#case) | case clause with break |
| [`class`](examples/javascript.md#class) | class definition |
| [`ctor`](examples/javascript.md#ctor) | constructor method |
| [`default`](examples/javascript.md#default) | default clause |
| [`doc`](examples/javascript.md#doc) | JSDoc comment line |
| [`el`](examples/javascript.md#el) | else block |
| [`elif`](examples/javascript.md#elif) | else-if block |
| [`exp`](examples/javascript.md#exp) | export a const binding |
| [`expd`](examples/javascript.md#expd) | export default |
| [`fn`](examples/javascript.md#fn) | function declaration |
| [`fore`](examples/javascript.md#fore) | for...of loop |
| [`fori`](examples/javascript.md#fori) | for loop over integer range |
| [`forin`](examples/javascript.md#forin) | for...in loop over object keys |
| [`get`](examples/javascript.md#get) | getter backed by a private-by-convention field |
| [`if`](examples/javascript.md#if) | if block |
| [`imp`](examples/javascript.md#imp) | default import |
| [`impd`](examples/javascript.md#impd) | destructured (named) import |
| [`jsonp`](examples/javascript.md#jsonp) | JSON.parse expression |
| [`jsons`](examples/javascript.md#jsons) | JSON.stringify expression |
| [`pr`](examples/javascript.md#pr) | console.log call |
| [`prd`](examples/javascript.md#prd) | debug-log with object-shorthand label |
| [`prom`](examples/javascript.md#prom) | new Promise with executor |
| [`ret`](examples/javascript.md#ret) | return statement |
| [`set`](examples/javascript.md#set) | setter backed by a private-by-convention field |
| [`switch`](examples/javascript.md#switch) | switch statement |
| [`test`](examples/javascript.md#test) | test case (jest/vitest style) |
| [`todo`](examples/javascript.md#todo) | TODO comment |
| [`try`](examples/javascript.md#try) | try/catch block |
| [`while`](examples/javascript.md#while) | while loop |

## typescript

id `typescript` | extensions: `ts`, `mts`, `cts`, `tsx` |
LSP ids: `typescript`, `typescriptreact` | default indent: 2 spaces |
inherits: the JavaScript pack

TypeScript starts from the JavaScript pack. Several triggers such as `afn` and
`fn` override the JavaScript versions to emit typed forms.

| Trigger | Description |
| --- | --- |
| [`aeq`](examples/typescript.md#aeq) | equality expectation |
| [`afn`](examples/typescript.md#afn) | typed async function declaration (overrides JS) |
| [`arrow`](examples/typescript.md#arrow) | arrow function bound to a const |
| [`asrt`](examples/typescript.md#asrt) | as-type assertion |
| [`aw`](examples/typescript.md#aw) | await statement |
| [`case`](examples/typescript.md#case) | case clause with break |
| [`class`](examples/typescript.md#class) | class definition |
| [`ctor`](examples/typescript.md#ctor) | constructor method |
| [`default`](examples/typescript.md#default) | default clause |
| [`doc`](examples/typescript.md#doc) | JSDoc comment line |
| [`el`](examples/typescript.md#el) | else block |
| [`elif`](examples/typescript.md#elif) | else-if block |
| [`enum`](examples/typescript.md#enum) | enum definition |
| [`exp`](examples/typescript.md#exp) | export a const binding |
| [`expd`](examples/typescript.md#expd) | export default |
| [`fn`](examples/typescript.md#fn) | typed function declaration (overrides JS) |
| [`fng`](examples/typescript.md#fng) | generic function declaration |
| [`fore`](examples/typescript.md#fore) | for...of loop |
| [`fori`](examples/typescript.md#fori) | for loop over integer range |
| [`forin`](examples/typescript.md#forin) | for...in loop over object keys |
| [`get`](examples/typescript.md#get) | getter backed by a private-by-convention field |
| [`if`](examples/typescript.md#if) | if block |
| [`imp`](examples/typescript.md#imp) | default import |
| [`impd`](examples/typescript.md#impd) | destructured (named) import |
| [`impt`](examples/typescript.md#impt) | type-only import |
| [`int`](examples/typescript.md#int) | interface definition |
| [`jsonp`](examples/typescript.md#jsonp) | JSON.parse expression |
| [`jsons`](examples/typescript.md#jsons) | JSON.stringify expression |
| [`pr`](examples/typescript.md#pr) | console.log call |
| [`prd`](examples/typescript.md#prd) | debug-log with object-shorthand label |
| [`prom`](examples/typescript.md#prom) | new Promise with executor |
| [`ret`](examples/typescript.md#ret) | return statement |
| [`set`](examples/typescript.md#set) | setter backed by a private-by-convention field |
| [`switch`](examples/typescript.md#switch) | switch statement |
| [`test`](examples/typescript.md#test) | test case (jest/vitest style) |
| [`todo`](examples/typescript.md#todo) | TODO comment |
| [`try`](examples/typescript.md#try) | try/catch block |
| [`typ`](examples/typescript.md#typ) | type alias |
| [`while`](examples/typescript.md#while) | while loop |

## go

id `go` | extensions: `go` | LSP ids: `go` | default indent: tab

| Trigger | Description |
| --- | --- |
| [`aeq`](examples/go.md#aeq) | equality check reporting through t.Errorf |
| [`case`](examples/go.md#case) | case clause |
| [`chan`](examples/go.md#chan) | channel binding |
| [`const`](examples/go.md#const) | const declaration |
| [`default`](examples/go.md#default) | default clause |
| [`defer`](examples/go.md#defer) | defer statement |
| [`doc`](examples/go.md#doc) | doc comment line |
| [`el`](examples/go.md#el) | else block (join onto the previous `}` by hand) |
| [`elif`](examples/go.md#elif) | else-if block (join onto the previous `}` by hand) |
| [`err`](examples/go.md#err) | errors.New expression |
| [`fn`](examples/go.md#fn) | function definition |
| [`fnr`](examples/go.md#fnr) | function with return type |
| [`for`](examples/go.md#for) | bare (infinite) for loop |
| [`forc`](examples/go.md#forc) | for loop with condition (Go's while) |
| [`fore`](examples/go.md#fore) | range loop over values |
| [`fori`](examples/go.md#fori) | for loop over integer range |
| [`forr`](examples/go.md#forr) | range loop with index and value |
| [`go`](examples/go.md#go) | goroutine over an anonymous function |
| [`if`](examples/go.md#if) | if block |
| [`iface`](examples/go.md#iface) | interface type definition |
| [`iferr`](examples/go.md#iferr) | if err != nil guard returning the error |
| [`imp`](examples/go.md#imp) | import block |
| [`let`](examples/go.md#let) | short variable declaration (:=) |
| [`main`](examples/go.md#main) | main entry point |
| [`map`](examples/go.md#map) | map binding |
| [`meth`](examples/go.md#meth) | method with receiver |
| [`pkg`](examples/go.md#pkg) | package clause |
| [`pr`](examples/go.md#pr) | fmt.Println call |
| [`prd`](examples/go.md#prd) | debug-print a value with %+v |
| [`prf`](examples/go.md#prf) | fmt.Printf with a format string and arguments |
| [`ret`](examples/go.md#ret) | return statement |
| [`sel`](examples/go.md#sel) | select statement (cases at select level, per gofmt) |
| [`struct`](examples/go.md#struct) | struct type definition |
| [`switch`](examples/go.md#switch) | switch statement (cases at switch level, per gofmt) |
| [`test`](examples/go.md#test) | test function (testing package) |
| [`todo`](examples/go.md#todo) | TODO comment |

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
