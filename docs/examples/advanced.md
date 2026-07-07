# Advanced examples

These abbreviations combine several composition operators at once — deep
nesting, climb-up, repetition, grouping, argument binding, and context-aware
variants — and each is broken down step by step. If any operator is
unfamiliar, read the [abbreviation syntax](../syntax.md) reference first; for
the full trigger list per language, see the [language reference](../languages.md).

> As elsewhere, examples use the default `strip` mode, which renders plain code
> and leaves an empty spot where each tabstop will be. A few examples also show
> `--tabstops keep` so you can see the `$1` / `${1:x}` / `$0` markers an editor
> steps through. See the [examples overview](README.md).

## An impl block with a deeply nested method, climbing back out to a sibling

```sh
codemmet expand --lang rust "impl:Ring>fn:push>fori10>ifl>prd^^^fn:len>ret"
```

```rust
impl Ring {
    fn push(&self) {
        for i in 0..10 {
            if let Some(value) = expr {
                println!("{:?}", );
            }
        }
    }
    fn len(&self) {
        return ;
    }
}
```

What's happening, operator by operator:

- `impl:Ring` opens the block; its `>` nests everything that follows as members.
- `fn:push` is the first member. Because a `fn` is composed **inside an `impl`**,
  codemmet selects the method variant of the `fn` trigger — that is why `push`
  gains `&self`. This is context awareness through composition alone; no editor
  or tree-sitter build is involved.
- `fori10>ifl>prd` nests three levels **inside** `push`: a counting `for` loop,
  an `if let` inside it, and a debug `println!` inside that. `fori10` glues the
  bound `10` onto the `fori` trigger; the loop variable stays a tabstop.
- `^^^` is the interesting part. After `prd` we are four levels deep (inside the
  `if let`, inside the `for`, inside `push`, inside `impl`). Each caret pops one
  nesting level: the first pops out of the `if let`, the second out of the
  `for`, the third out of `push`'s body — landing the next trigger back at the
  **impl's member level**.
- `fn:len` therefore becomes a *sibling method* of `push`, not something nested
  inside it. And because it too is now composed directly under the `impl`, it
  also gets `&self`. Its `>ret` nests a `return`.

Count the carets by counting the block boundaries you want to climb out of. Two
carets would have left `len` nested inside `push` as a local function (which is
valid Rust, just not what you usually want); three lands it as a method.

The same abbreviation in `keep` mode shows how tabstops are numbered across the
whole composed tree:

```sh
codemmet expand --lang rust --tabstops keep "impl:Ring>fn:push>fori10>ifl>prd^^^fn:len>ret"
```

```rust
impl Ring {
    fn push(&self$1) {
        for ${2:i} in 0..10 {
            if let ${3:Some(value)} = ${4:expr} {
                println!("{:?}", $5);
            }
        }
    }
    fn len(&self$6) {
        return $0;
    }
}
```

Tabstops are assigned in **document order**, not authoring order: `push`'s stops
(`$1`–`$5`) come before `len`'s (`$6`), even though `len` was written after the
climb-up. Exactly one `$0` survives as the final cursor — here it is the
innermost surviving stop, inside `len`'s `return`.

## Grouping a block of statements, then a sibling after the group

```sh
codemmet expand --lang rust 'fn:main>(letm:sum+fori10>prf:"i={}",i)+ret'
```

```rust
fn main() {
    let mut sum = ;
    for i in 0..10 {
        println!("i={}", i);
    }
    return ;
}
```

What's happening:

- `fn:main` opens the function; `>` nests its body.
- The parentheses group two statements: `letm:sum` (a `let mut sum` binding,
  with `sum` supplied as an explicit argument) and `fori10>prf:"i={}",i` (a loop
  whose body is a formatted `println!`). Grouping lets the trailing `+ret` apply
  to the whole group rather than just the last statement.
- `prf:"i={}",i` shows two explicit arguments at once: the format string
  `i={}` and the value `i`, bound positionally to `prf`'s parameters. (Note the
  single quotes around the whole abbreviation — `${…}`-free here, but the quotes
  also keep the shell from touching the `"` and `*`-free operators.)
- `+ret` is a sibling of the group, so `return` lands in `main` after the loop,
  not inside it.

Without the parentheses, `fn:main>letm:sum+fori10>prf...+ret` would still work
(all three become `main`'s children, since `>` binds the whole following group),
but the grouping makes the intent explicit and lets you attach `^`/`+`
operators to the group as a unit.

## A Python class: members as siblings, with a nested loop and try

```sh
codemmet expand --lang python "class:Cache>init+prop:size+def:get>fore:key>try"
```

```python
class Cache:
    def __init__(self):
        
    @property
    def size(self):
        return self._size
    def get(self):
        for key in iterable:
            try:
                
            except Exception as exc:
                
```

What's happening:

- `class:Cache` opens the class; `>` nests its members, joined with `+`:
  `init`, `prop:size`, and `def:get`.
- All three members pick up `self` because they are composed inside a `class`
  (the same context mechanism as `&self` in the Rust example). `prop:size` also
  mirrors the name into the body — `size` appears both as the method name and in
  `return self._size`.
- `def:get>fore:key>try` nests two levels inside the `get` method: a `for` loop
  over `key`, and a `try`/`except` inside the loop. The `except` clause comes
  from the `try` trigger's own body, so you get the full construct in one go.

## A Go function: the error-guard idiom plus a select loop

```sh
codemmet expand --lang go "fn:serve>let:addr+iferr+for>sel>case+default"
```

```go
func serve() {
	addr := 
	if err != nil {
		return err
	}

	for {
		select {
		case value:

		default:

		}
	}
}
```

What's happening:

- `fn:serve>` nests the body. Three siblings joined with `+`: a short variable
  declaration (`let:addr` → `addr := `), the classic `iferr` guard
  (`if err != nil { return err }`), and a `for` loop.
- `for>sel>case+default` nests a bare `for` containing a `select`, and the
  `select` holds a `case` and a `default` as siblings. Per gofmt, the `case` and
  `default` sit at the `select`'s indentation level rather than indented under
  it — codemmet emits them that way on purpose.
- Go indents with tabs, which is why every level above is a tab, matching what
  gofmt would produce.

## A TypeScript class: inherited constructor, typed async method, climb to a sibling

```sh
codemmet expand --lang typescript "class:Api>ctor+afn:fetch>try>aw^^fn:parse>ret"
```

```typescript
class Api {
  constructor() {
    
  }
  async fetch(): Promise<void> {
    try {
      await ;
    } catch (err) {

    }
  }
  parse(): void {
    return ;
  }
}
```

What's happening:

- `class:Api>ctor+afn:fetch` gives the class two members: a `constructor`
  (`ctor` is inherited from the JavaScript pack, since TypeScript builds on it)
  and an async method. `afn` is one of the triggers TypeScript **overrides** to
  emit a typed signature — hence `async fetch(): Promise<void>` rather than the
  untyped JavaScript form.
- `try>aw` nests a `try`/`catch` inside `fetch`, with an `await` statement inside
  the `try`.
- `^^fn:parse` climbs two levels: the first caret pops out of the `try` block,
  the second out of the `fetch` method — landing `parse` back at the class-member
  level. Because `parse` is now composed directly under the class, the `fn`
  trigger's method variant fires: it renders as `parse(): void` (a method, no
  `function` keyword), typed by the TypeScript pack.

## Repetition and climb-up together

```sh
codemmet expand --lang rust "fn:handle>match:msg>arm*3^ret"
```

```rust
fn handle() {
    match msg {
        pattern => ,
        pattern => ,
        pattern => ,
    }
    return ;
}
```

What's happening:

- `fn:handle>match:msg` nests a `match` in the function body.
- `arm*3` repeats the match-arm trigger three times, producing three arms as
  siblings inside the `match`.
- `^ret` climbs one level: the single caret pops out of the `match` body, so
  `return` becomes a sibling of the `match` — inside `handle`, after it. (Two
  carets would have popped out of `handle` too, putting `return` at the top
  level outside the function.)

In `keep` mode you can see that each repeated arm gets its **own independent
tabstops**, renumbered in document order, with the final cursor at the trailing
`return`:

```sh
codemmet expand --lang rust --tabstops keep "fn:handle>match:msg>arm*3^ret"
```

```rust
fn handle($1) {
    match msg {
        ${2:pattern} => $3,
        ${4:pattern} => $5,
        ${6:pattern} => $7,
    }
    return $0;
}
```

## A whole module: a struct, an impl with two methods, and a test module

This one scaffolds an entire module in a single abbreviation, combining every
operator: argument binding, nesting, grouping, siblings, climb-up, and repetition
with the `${i}` index.

```sh
codemmet expand --lang rust 'mod:shapes>((struct:Rect)+(impl:Rect>((fn:area>ret)+(fn:scale>fori:i,10>pr^ret)))+(tmod>test:case${i}*3))'
```

> Note the **single quotes**: `${i}` is the repetition-index variable, and a
> shell would try to expand `${i}` itself inside double quotes. See
> [troubleshooting](../troubleshooting.md).

```rust
mod shapes {
    struct Rect {
        
    }
    impl Rect {
        fn area(&self) {
            return ;
        }
        fn scale(&self) {
            for i in 0..10 {
                println!("");
            }
            return ;
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn case1() {
            
        }
        #[test]
        fn case2() {
            
        }
        #[test]
        fn case3() {
            
        }
    }
}
```

What's happening, from the outside in:

- `mod:shapes>` opens a module and nests everything after it inside.
- The outer group `((struct:Rect)+(impl:Rect>…)+(tmod>…))` holds the module's
  three members. **Each sibling that has its own children is wrapped in its own
  parentheses.** That is the key technique: a bare `A>x + B` would fold `B` in as
  another child of `A`, because `>` keeps attaching everything up to the next
  `)` or `^`. Wrapping each chain — `(A>x)+(B>y)` — makes them true siblings.
  (The [impl example above](#an-impl-block-with-a-deeply-nested-method-climbing-back-out-to-a-sibling)
  reaches the same shape with `^` climb-up instead; either works.)
- `struct:Rect` is the first member — a struct with a tabstop where its fields go.
- `impl:Rect>((fn:area>ret)+(fn:scale>…))` is the second member: an `impl` with
  two method siblings, again each parenthesized. Both methods gain `&self`
  because a `fn` composed **inside an `impl`** selects the method variant — this
  is context awareness from the composition scope alone, with no editor or
  tree-sitter involved.
- `fn:area>ret` nests a `return` in the first method's body.
- `fn:scale>fori:i,10>pr^ret` builds the second method's body: a `for` loop
  (`fori:i,10` binds the loop variable `i` and bound `10` explicitly) containing
  a `println!`, then `^` climbs out of the loop so the `return` lands **after**
  the loop, still inside `scale`.
- `tmod>test:case${i}*3` is the third member: the built-in test-module scaffold
  (`#[cfg(test)] mod tests { use super::*; … }`) with `test` repeated three
  times. `${i}` stamps the 1-based copy number into each name, so you get
  `case1`, `case2`, `case3` instead of three identically-named functions.

In `keep` mode the tabstops are numbered in document order across the entire
tree, with a single `$0` final cursor:

```sh
codemmet expand --lang rust --tabstops keep 'mod:shapes>((struct:Rect)+(impl:Rect>((fn:area>ret)+(fn:scale>fori:i,10>pr^ret)))+(tmod>test:case${i}*3))'
```

```rust
mod shapes {
    struct Rect {
        $1
    }
    impl Rect {
        fn area(&self$2) {
            return $3;
        }
        fn scale(&self$4) {
            for i in 0..10 {
                println!("$5");
            }
            return $6;
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn case1() {
            $7
        }
        #[test]
        fn case2() {
            $8
        }
        #[test]
        fn case3() {
            $0
        }
    }
}
```

From one line of abbreviation, that is a 33-line module skeleton with nine
tabstops to jump through — the struct body first (`$1`), then each method and
its `return`, then the three test bodies, ending on `$0` in the last test.

## Data-driven repeats: one arm per enum variant

`*3` repeats a fixed number of times; `*@Name` repeats over a **collection** and
fills each copy from it. The `arm` snippet renders `${value}` as the arm
pattern, so a `match` writes one arm per variant. On the CLI you supply the
collection with `--repeat`; in an editor codemmet reads it from a same-file
`enum` automatically (just `match:event>arm*`).

```sh
codemmet expand --lang rust "match:event>arm*@Event" \
    --repeat Event=Click,Scroll,KeyPress
```

```rust
match event {
    Click => ,
    Scroll => ,
    KeyPress => ,
}
```

In `keep` mode each arm still gets its own tabstop for the body, so you can jump
straight through filling them in:

```sh
codemmet expand --lang rust --tabstops keep "match:event>arm*@Event" \
    --repeat Event=Click,Scroll,KeyPress
```

```rust
match event {
    Click => $1,
    Scroll => $2,
    KeyPress => $0,
}
```

## Defining an enum, then a struct, from the same collection idiom

The `variant` and `field` snippets are the definition-side mirror of `arm`.
`variant` fills enum members and `field` fills struct fields — and a field
carries its declared type on `${value.type}`, supplied as `value:type`:

```sh
codemmet expand --lang rust "enum:Event>variant*@Event" \
    --repeat Event=Click,Scroll,KeyPress
```

```rust
enum Event {
    Click,
    Scroll,
    KeyPress,
}
```

```sh
codemmet expand --lang rust "struct:Config>field*@Cfg" \
    --repeat Cfg=host:String,port:u16
```

```rust
struct Config {
    host: String,
    port: u16,
}
```

So one buffer can drive the whole cycle: define `enum Event { … }` with
`variant*`, then elsewhere consume it with `match:event>arm*` — which reads
those variants back out. See
[context-aware & data-driven expansion](../context-codegen.md) for the sources
codemmet extracts and the per-language coverage.

## See also

- [Examples overview](README.md) and the per-language pages.
- [Abbreviation syntax](../syntax.md) — the grammar and operator precedence.
- [Context-aware & data-driven expansion](../context-codegen.md) — repeats
  driven by the surrounding code.
- [Tutorials](../tutorials.md) — build up from the basics.
- [Language reference](../languages.md) — every trigger, linked to its own example.
