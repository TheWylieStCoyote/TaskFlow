# Tutorials

These tutorials build on each other. Work through them in order with the
`codemmet` binary in front of you; each step shows the exact command and the
output you should see. If you have not installed codemmet yet, follow the
[installation](installation.md) guide first.

Every command below runs against the builtin snippet packs. Terms used
throughout — **abbreviation**, **trigger**, **expansion**, **tabstop**,
**glued argument**, **explicit argument**, **composition operator** — are
defined in the [abbreviation syntax](syntax.md) reference; this page introduces
them by example.

## 1. Your first expansion

An **abbreviation** is the terse string you type. A **trigger** is a registered
keyword inside it. The `expand` command turns an abbreviation into code:

```sh
codemmet expand --lang rust "fn:main"
```

```text
fn main() {
    
}
```

`fn` is the trigger. The `main` after the colon is an **explicit argument** that
fills the function name. Notice the empty indented line: that is where your
cursor lands, ready for a body.

Now grow the abbreviation. The `>` **composition operator** nests one trigger
inside another:

```sh
codemmet expand --lang rust "fn:main>pr"
```

```text
fn main() {
    println!("");
}
```

`pr` (Rust's `println!`) is now the child of `fn:main`, placed exactly where the
body goes. You just composed two triggers into one expansion.

## 2. Nesting and the flagship example

Chain three triggers with `>` to build a whole function at once:

```sh
codemmet expand --lang rust "fn:main>fori10>pr"
```

```text
fn main() {
    for i in 0..10 {
        println!("");
    }
}
```

Each `>` nests deeper, and codemmet handles the indentation for you.

So far the output has been plain text. That is the default, called **strip**
mode: every tabstop marker is removed and each placeholder becomes its plain
text. Editors want the markers left in so you can Tab between edit points. Ask
for that with `--tabstops keep`:

```sh
codemmet expand --lang rust "fn:main>fori10>pr" --tabstops keep
```

```text
fn main($1) {
    for ${2:i} in 0..10 {
        println!("$0");
    }
}
```

The `$1`, `${2:i}`, and `$0` are **tabstops**. `$1` is the first stop (the
argument list), `${2:i}` is a stop pre-filled with the placeholder `i`, and `$0`
is where the cursor finishes. codemmet renumbers the tabstops across the whole
composed expansion so they run in one clean sequence. Strip mode simply drops
all of this and leaves the plain text you saw first.

## 3. Arguments: glued and explicit

Triggers take two kinds of argument. A **glued argument** is written directly
after the trigger; an **explicit argument** follows a `:` or `,`.

The `10` in `fori10` is a glued argument. The `fori` trigger declares a numeric
parameter, so the glued `10` binds to it:

```sh
codemmet expand --lang rust "fori10"
```

```text
for i in 0..10 {
    
}
```

The `main` in `fn:main` is an explicit argument, supplied after the colon:

```sh
codemmet expand --lang rust "fn:main"
```

```text
fn main() {
    
}
```

A trigger can take several explicit arguments separated by commas. Rust's `prf`
takes a format string and its arguments:

```sh
codemmet expand --lang rust 'prf:"i={}",i'
```

```text
println!("i={}", i);
```

The quotes are part of the argument value, so wrap the whole abbreviation in
single quotes to protect them from your shell.

## 4. Siblings, repeats, groups, and climbing up

The other **composition operators** let you assemble more than a single chain.

`+` places two triggers as **siblings** at the same level:

```sh
codemmet expand --lang rust "if+el"
```

```text
if condition {
    
}
else {
    
}
```

`*n` **repeats** a trigger. Here three match arms are generated inside a
`match`:

```sh
codemmet expand --lang rust "match:cmd>arm*3"
```

```text
match cmd {
    pattern => ,
    pattern => ,
    pattern => ,
}
```

In keep mode you can see each repeat gets its own tabstops:

```sh
codemmet expand --lang rust "match:cmd>arm*3" --tabstops keep
```

```text
match cmd {
    ${1:pattern} => $2,
    ${3:pattern} => $4,
    ${5:pattern} => $0,
}
```

`^` **climbs up** one level, so the next trigger is a sibling of the parent
rather than a child. Here `ret` climbs out of the loop to sit at the function
body level:

```sh
codemmet expand --lang rust "fn:run>fori10>pr^ret"
```

```text
fn run() {
    for i in 0..10 {
        println!("");
    }
    return ;
}
```

Parentheses `( )` group a sub-abbreviation so an operator applies to the whole
group; see the [abbreviation syntax](syntax.md) reference for grouped examples.

## 5. Other languages

The same abbreviation grammar works across every supported language; only the
generated code and its indentation change. Pick the language with `--lang`,
which accepts a language id, a file extension (`rs`, `py`, `go`), or an editor's
LSP language id.

A Python class with an `__init__` and a `__str__` as siblings:

```sh
codemmet expand --lang python "class:Point>init+str"
```

```text
class Point:
    def __init__(self):
        
    def __str__(self) -> str:
        return f""
```

Go uses tab indentation, and its `iferr` trigger expands the idiomatic error
check:

```sh
codemmet expand --lang go "fn:run>iferr"
```

```text
func run() {
	if err != nil {
		return err
	}
	
}
```

Python indents with four spaces, Go with tabs, and TypeScript with two spaces —
codemmet renders each language in its own style from the same triggers. See the
[language reference](languages.md) for the full trigger list per language.

## 6. Wrapping existing code

`wrap` takes code on standard input and wraps it in an expansion, placing the
input where the `${child}` slot would be. This wraps two lines in a `for` loop:

```sh
printf 'let x = compute();\nprintln!("{x}");' | codemmet wrap --lang rust --abbr fori10 --tabstops keep
```

```text
for ${1:i} in 0..10 {
    let x = compute();
    println!("{x}");$0
}
```

The wrapped lines are re-indented to match their new depth. Drop `--tabstops
keep` for plain output with the tabstops stripped.

## 7. Piping and scripting

`expand` reads the abbreviation from standard input when you pass `-` (or omit
it), which makes it easy to drive from other tools:

```sh
printf 'fori10' | codemmet expand --lang rust -
```

```text
for i in 0..10 {
    
}
```

For programmatic use, `--json` prints one object with the text, the cursor
position, and whether the result contains tabstops:

```sh
codemmet expand --lang rust --json --tabstops keep "fori10"
```

```text
{"cursor":null,"is_snippet":true,"text":"for ${1:i} in 0..10 {\n    $0\n}"}
```

`expand` also sets a meaningful exit code, so scripts can branch on failure:

| Exit code | Meaning |
| --- | --- |
| `0` | Success |
| `1` | Parse error in the abbreviation |
| `2` | Unknown trigger, or the expansion could not be rendered |
| `3` | Configuration or language error (for example an unknown `--lang`) |

A small loop that reports each result:

```sh
for abbr in pr nope fori10; do
  codemmet expand --lang rust "$abbr" >/dev/null 2>&1
  echo "$abbr -> exit $?"
done
```

```text
pr -> exit 0
nope -> exit 2
fori10 -> exit 0
```

The full flag list is in the [CLI reference](cli.md).

## 8. Using it in your editor

In day-to-day use you will not run `expand` by hand — the LSP server does it as
you type. The flow is:

1. Type an abbreviation trigger such as `fori10` in a source file.
2. Accept the completion the editor offers for it.
3. The expansion is inserted in **keep** mode, and your cursor jumps to the
   first tabstop.
4. Press Tab to move through `$1`, `$2`, and so on, editing each placeholder,
   until you reach the final `$0`.

To discover what is available for a language, list every trigger:

```sh
codemmet list --lang rust
```

```text
aeq     	assert_eq! statement
afn     	async function definition
arm     	match arm
...
```

For the real setup — installing the LSP server and the Neovim plugin, and
configuring completion — see [editor setup](editors.md).

## 9. Making your own snippet

When a builtin does not fit, define your own trigger. A user snippet pack is a
small TOML file in your config directory; a trigger you add is merged over the
builtins, and one you redefine replaces the builtin of that name. Validate the
directory any time with:

```sh
codemmet check
```

```text
ok: 7 languages, 251 triggers (user snippets: /home/you/.config/codemmet/snippets)
```

The full authoring guide — schema, parameters, body syntax, overriding,
disabling, and context-aware variants — is in [custom snippets](snippets.md).

## Where to go next

- [abbreviation syntax](syntax.md) — the complete grammar and resolution rules
- [language reference](languages.md) — every builtin trigger, per language
- [custom snippets](snippets.md) — author and override your own packs
- [CLI reference](cli.md) — all commands and flags
- [editor setup](editors.md) — the LSP server and Neovim plugin
- [troubleshooting](troubleshooting.md) — when something does not expand
- [documentation home](README.md)
