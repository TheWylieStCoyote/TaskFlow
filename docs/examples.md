# Examples cookbook

A collection of longer, real-world abbreviations that combine several
composition operators at once. Each one was produced by the shipped binary;
the commands use `--config /tmp/codemmet-none` only to guarantee that no local
snippet overrides affect the output — you can drop that flag.

These examples lean on the full grammar. If an operator is unfamiliar, read the
[abbreviation syntax](syntax.md) reference first: `>` nests a child, `+` adds a
sibling, `*` repeats, `^` climbs back up out of a nesting level, and `()`
groups.

> **A note on the empty spots.** These examples use the default `strip` mode,
> which renders plain code and leaves an empty spot wherever a tabstop would be
> — `assert_eq!(left, )`, `data := `, and so on. Those are the places you fill
> in; in an editor they become tabstops you jump through with Tab. Add
> `--tabstops keep` to see them as `$1` / `${1:placeholder}` / `$0` markers
> instead. The last section shows the difference.

## Rust: an `impl` block with several methods

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

What is happening: `impl:Counter` opens the block and its `>` nests the first
method inside it. `fn:increment>pr` puts a `println!` in that method's body.
The `^` then climbs back out of the first method so `fn:value` is its sibling,
not nested inside it. Notice both functions gained `&self`: codemmet knows a
`fn` composed inside an `impl` is a method, so it selects the method variant of
the `fn` trigger. This context awareness works through composition alone — no
editor or tree-sitter build required. See the
[language reference](languages.md) for which triggers are context-aware.

## Rust: a test module with two tests

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

`tmod` is the `#[cfg(test)] mod tests { … }` scaffold; each `test:<name>` nests
a `#[test]` function with an `assert_eq!` inside it, and the `^` between them
makes the second test a sibling of the first rather than nesting it inside.

## Rust: deep nesting, then climbing all the way out

```sh
codemmet expand --lang rust 'fn:process>fori10>ifl>prf:"item {}",value^^ret'
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

This nests three levels deep — function, then loop, then `if let` — and drops a
formatted `println!` at the bottom. The double `^^` then climbs back up *two*
levels so the `return` lands directly in the function body, past both the `if`
and the `for`. Each caret pops one level; stack as many as you need (extra
carets simply stop at the top). The `prf:"item {}",value` also shows explicit
arguments: a quoted format string and a value, bound positionally.

## Python: a class with a constructor, a property, and a dunder

```sh
codemmet expand --lang python "class:Account>init+prop:balance+str"
```

```python
class Account:
    def __init__(self):
        
    @property
    def balance(self):
        return self._balance
    def __str__(self) -> str:
        return f""
```

`class:Account` opens the class and the `>` nests its members; the three
members are joined with `+` so they sit side by side in the body. `prop:balance`
even mirrors the name into the return line (`self._balance`) — a parameter used
twice in a snippet body becomes a tabstop plus a mirror.

## Python: a nested loop with error handling

```sh
codemmet expand --lang python "def:sync>fore:item>try"
```

```python
def sync():
    for item in iterable:
        try:
            
        except Exception as exc:
            
```

A straight chain of `>` operators: function, then a `for … in` loop, then a
`try`/`except` inside the loop body.

## Go: an idiomatic function with an error guard

```sh
codemmet expand --lang go "fn:load>let:data+iferr+ret"
```

```go
func load() {
	data := 
	if err != nil {
		return err
	}

	return 
}
```

Three siblings inside the function body, joined with `+`: a short variable
declaration, the classic `if err != nil { return err }` guard, and a final
`return`. Go's expansions indent with tabs, matching `gofmt`.

## TypeScript: a class with a constructor and an async method

```sh
codemmet expand --lang typescript "class:Service>ctor+afn:fetch>try"
```

```typescript
class Service {
  constructor() {
    
  }
  async fetch(): Promise<void> {
    try {
      
    } catch (err) {
      
    }
  }
}
```

The class holds two members (`+`): a constructor and an async method, and the
method nests a `try`/`catch` (`>`). TypeScript indents with two spaces and its
`afn` trigger produces a typed signature (`: Promise<void>`) — that is the
TypeScript pack overriding the JavaScript one it inherits from.

## Wrapping existing code

Wrap takes a selection on stdin and injects it into the expansion. This is how
you retrofit a construct around code you already wrote.

Wrap two lines in a `for` loop (shown in `keep` mode so you can see where the
cursor lands):

```sh
printf 'let total = items.iter().sum();\nprintln!("{total}");' \
  | codemmet wrap --lang rust --tabstops keep --abbr "fori10"
```

```rust
for ${1:i} in 0..10 {
    let total = items.iter().sum();
    println!("{total}");$0
}
```

Wrap a block in an `if let`:

```sh
printf 'let config = load_config();\nrun(config);' \
  | codemmet wrap --lang rust --abbr "ifl"
```

```rust
if let Some(value) = expr {
    let config = load_config();
    run(config);
}
```

The selection is re-indented one level deeper and dropped in at the
construct's cursor position. In an editor you do this by selecting lines and
triggering wrap; see [editor setup](editors.md).

## Seeing the tabstops

Everything above used `strip` mode. The same abbreviation in `keep` mode shows
the tabstops an editor would step through:

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

`$1` is where the cursor lands first (the method's parameter list, after
`&self`), and `$0` is the final resting place (inside the `return`). codemmet
renumbers tabstops across the whole composed expansion, so no matter how many
triggers you chain together the stops stay in a sensible order. See the
[abbreviation syntax](syntax.md) reference for how tabstops and mirrors work.

## See also

- [Tutorials](tutorials.md) — build up to these step by step.
- [Abbreviation syntax](syntax.md) — the full grammar.
- [Language reference](languages.md) — every trigger, per language.
- [Custom snippets](snippets.md) — define your own triggers to compose.
