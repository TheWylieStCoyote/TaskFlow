# codemmet examples

Worked, verified examples of codemmet abbreviations. Each per-language page
below collects longer abbreviations that combine several composition operators
at once; this page covers the cross-cutting techniques (wrapping existing code,
and reading tabstops) and links out to the rest.

If an operator is unfamiliar, read the [abbreviation syntax](../syntax.md)
reference first: `>` nests a child, `+` adds a sibling, `*` repeats, `^` climbs
back up out of a nesting level, and `()` groups.

## By language

| Language | Examples |
| --- | --- |
| [Rust](rust.md) | structs, impl blocks, traits, `match`, deep nesting with climb-up, test modules |
| [Python](python.md) | classes, properties, `__main__` guard, pattern matching, `with`, nested loops |
| [C](c.md) | `main` loops, nested counters, `switch`, structs, allocation, `do`/`while` |
| [C++](cpp.md) | class methods, namespaces, `std::vector`, templates, smart pointers, maps |
| [JavaScript](javascript.md) | classes, async/await, getters and setters, Promises, `switch`, imports |
| [TypeScript](typescript.md) | interfaces, typed methods, generics, type aliases, enums |
| [Go](go.md) | error guards, structs and methods, ranges, goroutines, `select`, interfaces, tests |

## Reading the output

The examples use the default `strip` mode, which renders plain code and leaves
an empty spot wherever a **tabstop** would be — `assert_eq!(left, )`, `data :=`,
and so on. Those are the places you fill in. In an editor they become the stops
you jump through with Tab. Add `--tabstops keep` to any command to see them as
`$1` / `${1:placeholder}` / `$0` markers instead; see the [abbreviation
syntax](../syntax.md) reference for how tabstops and mirrors work.

The commands below assume the built-in snippet packs. If you have added your own
[custom snippets](../snippets.md), point `--config` at an empty directory to
reproduce the exact output shown here.

## Wrapping existing code

`codemmet wrap` takes a selection on stdin and injects it into the expansion at
the cursor position. This is how you retrofit a construct around code you have
already written; it works for every language.

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

Wrap a block in an `if let` (default `strip` mode):

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

The selection is re-indented one level deeper and dropped in at the construct's
cursor position. In an editor you do this by selecting lines and triggering
wrap; see [editor setup](../editors.md).

## See also

- [Tutorials](../tutorials.md) — build up to these step by step.
- [Abbreviation syntax](../syntax.md) — the full grammar.
- [Language reference](../languages.md) — every trigger, per language.
- [Custom snippets](../snippets.md) — define your own triggers to compose.
