# Abbreviation syntax

An **abbreviation** is the terse string you hand to codemmet, and the code it
generates is the **expansion**. The building blocks of an abbreviation are
**triggers** (registered keywords like `fori` or `fn`) joined together with
**composition operators**. codemmet reads the abbreviation, resolves each
trigger, fills in any arguments you supplied, and prints the resulting code.

Here is a flagship example. The abbreviation `fn:main>fori10>pr` reads as
"a function named `main`, containing a for-loop over `0..10`, containing a
`println!`":

```sh
codemmet expand --lang rust --tabstops keep "fn:main>fori10>pr"
```

```rust
fn main($1) {
    for ${2:i} in 0..10 {
        println!("$0");
    }
}
```

The rest of this page explains each piece: the composition operators, how to
pass arguments, what the `$1`/`$0` markers are, and how expansion can change
based on surrounding code.

## The grammar

Every abbreviation follows this small grammar. Read it top to bottom; each
line names a construct and the line's right side shows how it is built:

```text
abbr    := group
group   := level ( '^'+ level )*        ; '^' climbs up out of the current nesting
level   := chain ( '+' chain )*         ; '+' = sibling
chain   := element ( '>' level )?       ; '>' = nest the following as a child
element := atom ( '*' NUMBER )?         ; '*' = repeat
atom    := node | '(' group ')'         ; parentheses group
node    := WORD args?
args    := ':' arg ( ',' arg )*         ; explicit arguments
```

A `WORD` matches the pattern `[A-Za-z_][A-Za-z0-9_]*` (a letter or underscore
followed by letters, digits, or underscores). codemmet resolves a `WORD` by
**longest-prefix match** against the current language's triggers. In `fori10`,
the longest trigger that prefixes the word is `fori`, so codemmet treats the
`fori` as the trigger and the trailing `10` as a **glued argument**. A bare
`fn` simply matches the `fn` trigger.

### Precedence

The composition operators bind with the following precedence, tightest first:

| Rank | Operator | Meaning |
| --- | --- | --- |
| 1 (tightest) | `:` and `,` | arguments to a trigger |
| 2 | `*` | repeat |
| 3 | `>` | child (nesting) |
| 4 (loosest) | `+` | sibling |

Parentheses `(` `)` override this and group whatever they enclose. These are
the same rules Emmet uses. In `a>b+c` both `b` and `c` are children of `a`,
because `>` binds tighter than `+`. In `a+b>c`, the `>` binds tighter than the
`+`, so `c` is a child of `b`, and `a` is a sibling of `b`.

## Composition operators

### `>` — child (nesting)

The greater-than sign nests what follows it inside the current construct's body
slot. In `fn:main>fori10`, the for-loop is placed inside the body of `main`:

```sh
codemmet expand --lang rust --tabstops keep "fn:main>fori10>pr"
```

```rust
fn main($1) {
    for ${2:i} in 0..10 {
        println!("$0");
    }
}
```

Each `>` descends one more level: the `println!` here sits inside the loop,
which sits inside the function.

### `+` — sibling

The plus sign places two constructs at the same level, one after another. The
abbreviation `if+el` produces an if block followed by an else block as
siblings:

```sh
codemmet expand --lang rust "if+el"
```

```rust
if  {
    
} else {
    
}
```

### `*` — repeat

An asterisk followed by a number repeats the preceding element. The
abbreviation `arm*3` produces three match arms:

```sh
codemmet expand --lang rust "match:cmd>arm*3"
```

```rust
match cmd {
    pattern => ,
    pattern => ,
    pattern => ,
}
```

You can repeat a whole group too: `(fori10>pr)*2` produces two copies of the
loop-with-body. Repeat counts are capped, so very large repeats are rejected.

#### Data-driven repeats: `*@name`, `*`, and `*[…]`

Instead of a fixed count, a repeat can be driven by a **collection** — one copy
per item. `*@name` draws from a named collection, a bare `*` from the enclosing
construct's subject, and `*[a,b,c]` from an **inline literal** written right in
the abbreviation (each value may carry a member as `value:type`). The current
item names the construct — it auto-binds to the first empty argument — or fills
`${value}` in the body:

```sh
codemmet expand --lang rust "impl:Point>fn*[new,len,area]"
```

```rust
impl Point {
    fn new(&self) {  }
    fn len(&self) {  }
    fn area(&self) {  }
}
```

These forms only apply to a single node (not a group or composite). See the
[context-aware & data-driven guide](context-codegen.md) for the full model and
the `${value}` details.

#### The repetition index `${i}`

Within a repeated node, the variable `${i}` expands to the **1-based copy
number** — `1`, `2`, `3`, and so on (and just `1` when the node is not
repeated). It works both inside an explicit argument and inside a snippet
body. For example, `test:case${i}*3` names three test functions in order:

```sh
codemmet expand --lang rust 'test:case${i}*3'
```

> **Quote it with single quotes.** `${i}` looks like a shell variable, so in a
> double-quoted argument bash and zsh replace it with the (usually empty)
> variable `i` before codemmet runs — you would get `test:case*3` and three
> functions all named `case`. Single quotes pass `${i}` through literally. In
> an editor this does not apply; you type the abbreviation directly.

```rust
#[test]
fn case1() {
    
}
#[test]
fn case2() {
    
}
#[test]
fn case3() {
    
}
```

Snippet authors can use `${i}` directly in a body (see [custom
snippets](snippets.md)); `i` is a reserved parameter name because of it. The
index tracks each node's own repeat, so it applies to `node*n`, not to a
group written as `(...)*n`.

#### Data-driven repeats (`*@name`)

Instead of a fixed count, a repeat can be driven by a **collection** — one copy
per item, with each copy filled from that item. Write `*@name` to repeat over a
named collection:

```sh
# one match arm per variant of the Msg enum:
codemmet expand --lang rust "match:cmd>arm*@Msg"
```

A snippet body pulls the current item in with `${value}`; a member of the item
with `${value.type}` (or any `${value.key}`); and supplies a fallback for when
there is no data with `${value:default}`. Three built-in snippets use these:
`arm` (a match arm), `field` (a struct/record field, with its declared type),
and `variant` (an enum member — the definition side). So an enum fills its
arms, a struct fills its fields, and — going the other way — a fresh `enum`
fills its own variants:

```rust
match cmd {
    Quit => ,
    Move => ,
    Write => ,
}
```

Defining an enum is the mirror image — `variant` nests inside `enum`, so
`enum:Color>variant*3` scaffolds three variants (each a `Name,` tabstop), and
`enum:Color>variant*@Src` fills them from a collection. `field` behaves the
same inside `struct`.

Where does the collection come from?

- **In an editor**, codemmet reads it from the buffer with tree-sitter. Three
  kinds are recognized: an **enum**'s variants (`match:Msg>arm*@Msg`), a
  **struct**/record's fields with their declared types
  (`struct:Wrapper>field*@Point` → `x: i32,` / `y: f64,`), and an **in-scope
  list** binding's elements (`let items = [a, b, c]` → `arm*@items`). Enum and
  struct definitions are visible anywhere in the file; a list binding must be
  in scope and appear before the cursor. This works across all seven languages
  where the construct exists (JavaScript has lists only).
- **On the CLI**, there is no buffer, so you supply collections with
  `--repeat name=item1,item2,…` (repeatable). Each item may carry a member
  after a colon — `field*@Point --repeat Point=x:i32,y:f64` gives each item a
  `type`:

  ```sh
  codemmet expand --lang rust "struct:Wrapper>field*@Point" \
      --repeat Point=x:i32,y:f64
  ```

If the enclosing construct already names the source — `match:Msg` — a bare
`arm*` (no `@`) repeats over that subject, so `match:Msg>arm*` is shorthand for
`match:Msg>arm*@Msg`. When a data-driven repeat finds no matching collection it
degrades to a single copy with the `${value:default}` fallbacks, so `arm*` is
always safe to type.

> Extraction is **shape-directed, not type-aware**: codemmet reads what the
> syntax literally says (an enum's written variants, a list literal's written
> elements). It does not infer a variable's type to find its definition — you
> name the source (`@Msg`). `value` is a reserved parameter name because of
> `${value}`.

### `^` — climb-up

A caret ends the current nesting level and attaches what follows one level
higher. In `fn:main>fori10>pr^ret`, the `println!` goes inside the loop, then
the `^` climbs back out so that `ret` (a return statement) becomes a sibling of
the loop, inside `main`:

```sh
codemmet expand --lang rust "fn:run>letm:total+fori10>pr^ret"
```

```rust
fn run() {
    let mut total = ;
    for i in 0..10 {
        println!("");
    }
    return ;
}
```

Multiple carets climb multiple levels at once. If you write more carets than
there are levels to climb, the extras clamp harmlessly at the top level.

### `(` `)` — group

Parentheses group a run of constructs so that operators outside the parentheses
apply to the group as a whole. In `(fori10>pr)+ret`, the parentheses bundle the
loop and its body together, so the `+ret` that follows makes `ret` a sibling of
the entire group rather than a sibling of the `println!` alone.

## Arguments

Most triggers declare parameters (a function name, a loop bound, a format
string). There are two ways to fill them.

### Glued arguments

A **glued argument** is typed directly after the trigger, with no separator. A
numeric glued argument binds to the trigger's first integer parameter, and an
identifier glued argument binds to its first identifier parameter. In `fori10`,
the `10` sets the loop's upper bound to 10, while the loop variable is left as a
tabstop for you to fill in.

### Explicit arguments

**Explicit arguments** start with a colon `:` and are separated by commas `,`.
They bind positionally, in the order the trigger declares its parameters.
`fn:main` names the function `main`. You can pass several at once:

```sh
codemmet expand --lang rust 'prf:"i={}",i'
```

```rust
println!("i={}", i);
```

Here `prf` receives two arguments: the quoted format string `"i={}"` and the
value `i`.

### Quoting and escaping

Quote an argument with `"..."` or `'...'` when it contains spaces, commas, or
operator characters that would otherwise be read as part of the abbreviation
grammar. Inside a raw (unquoted) argument, a backslash escapes the next
character.

## Tabstops and keep/strip

Any parameter you do not supply becomes a **tabstop**: a landing spot an editor
jumps through in sequence. Tabstops are written like this:

| Marker | Meaning |
| --- | --- |
| `$1`, `$2`, ... | ordered stops the editor visits in turn |
| `${1:name}` | a stop that carries `name` as a placeholder |
| `$0` | the final cursor position |

The CLI's `--tabstops` flag controls how tabstops are rendered. With
`--tabstops keep`, codemmet prints the raw tabstop markers, which is what an
editor consumes:

```rust
fn main($1) {
    for ${2:i} in 0..10 {
        println!("$0");
    }
}
```

With `--tabstops strip` (the plain-text form), codemmet removes the markers and
leaves only each tabstop's default text. When a tabstop has no default, this
leaves an empty spot, which is expected:

```rust
fn run() {
    let mut total = ;
    for i in 0..10 {
        println!("");
    }
    return ;
}
```

The `let mut total = ;` line looks unfinished because the value was never
supplied. In an editor those blanks are the landing spots you type into; in
strip mode you simply see where they are. See the [CLI reference](cli.md) for
the full set of flags.

## Context-aware expansion

Inside an editor, the same trigger can expand differently depending on the code
that surrounds it. For example, Rust's `fn` gains a `&self` parameter when it
appears inside an `impl` block, and Python's `def` gains a `self` parameter
inside a `class`. codemmet detects this using tree-sitter, so it applies only
on the editor and language-server path (including composed forms such as
`impl:Point>fn:new`), not to plain CLI expansion. This is the same mechanism
that lets a data-driven repeat read enum variants and struct fields from the
buffer — see [context-aware & data-driven expansion](context-codegen.md) for
the whole picture.

## See also

- [documentation home](README.md)
- [context-aware & data-driven expansion](context-codegen.md)
- [language reference](languages.md)
- [CLI reference](cli.md)
- [custom snippets](snippets.md)
- [editor setup](editors.md)
- [tutorials](tutorials.md)
- [troubleshooting](troubleshooting.md)
