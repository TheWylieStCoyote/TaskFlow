# C examples

The C pack has language id `c` and matches on the `c` and `h` file extensions or the LSP id `c`, so `--lang c` and `--lang mycode.h` both select it. It indents with 4 spaces by default and is the base pack that [C++](cpp.md) inherits from; see the [language reference](../languages.md) for the full list of triggers.

> Examples use the default `strip` mode, which renders plain code and leaves an empty spot wherever a tabstop will be — those are the places you fill in. Add `--tabstops keep` to see them as `$1` / `${1:placeholder}` / `$0` markers. See the [examples overview](README.md) for more.

## A main function with a counted loop

```sh
codemmet expand --lang c "main>fori10>prf:\"%d\n\",i"
```

```c
int main(void) {
    for (int i = 0; i < 10; i++) {
        printf("%d\n", i);
    }
    return 0;
}
```

The `main` trigger includes its own `return 0;`, and the `>` child operator nests everything else inside its body. `fori10` glues the argument `10` to become the loop bound, while `prf` takes an explicit format string and value through the glued arguments `"%d\n",i`.

## Nested loops with named counters

```sh
codemmet expand --lang c "main>fori:i,3>fori:j,3>prf:\"%d \",i"
```

```c
int main(void) {
    for (int i = 0; i < 3; i++) {
        for (int j = 0; j < 3; j++) {
            printf("%d ", i);
        }
    }
    return 0;
}
```

Each `fori` takes explicit arguments with `:` — `:i,3` and `:j,3` name the loop variable and bound, so the inner loop counts with `j`. Chaining the `>` child operator nests the second loop and the `prf` call inside the first.

## A switch with cases and a default

```sh
codemmet expand --lang c "switch:cmd>case+case+default"
```

```c
switch (cmd) {
    case value:
        
        break;
    case value:
        
        break;
    default:
        
        break;
}
```

The `switch` trigger takes an explicit argument (`:cmd`) for the tested expression, and the `+` sibling operator lays out two `case` blocks and a `default` at the same level. Each `case` ships with its own `break;`.

## A typedef'd struct

```sh
codemmet expand --lang c "struct:Point"
```

```c
typedef struct {
    
} Point;
```

The `struct` trigger emits a `typedef struct`, using the explicit argument `:Point` as the type name and leaving a tabstop for the fields.

## Allocating a buffer

```sh
codemmet expand --lang c "fn:dup>malloc+ret"
```

```c
void dup() {
    int *ptr = malloc(1 * sizeof(int));
    return ;
}
```

The `fn` trigger names the function through its explicit argument `:dup`, and the `>` child operator places both statements in the body. `malloc` and `ret` are joined with the `+` sibling operator so they appear one after another.

## A do-while loop

```sh
codemmet expand --lang c "fn:wait>dowhile"
```

```c
void wait() {
    do {
        
    } while (condition);
}
```

The `dowhile` trigger expands to a full `do { } while` loop, nested inside the `fn` body by the `>` child operator, with tabstops for the body and the condition.

## Trigger reference

One entry per C trigger, alphabetical. Block and named triggers show both a simple form and a composed form that nests a child with the `>` operator; leaf triggers show a single form. Outputs are shown in the default `strip` mode, so tabstops render as empty gaps.

### `aeq`

Equality assertion.

```sh
codemmet expand --lang c "aeq"
```

```c
assert(actual == );
```

`aeq` has no parameters; it expands to an `assert` comparing two tabstops, with `actual` as the first placeholder and the expected value left empty.

### `assert`

Assert statement.

```sh
codemmet expand --lang c "assert"
```

```c
assert();
```

`assert` takes no arguments and leaves a single tabstop inside the call for the condition.

### `case`

Case label with break.

```sh
codemmet expand --lang c "case"
```

```c
case value:
    
    break;
```

```sh
codemmet expand --lang c "case>ret"
```

```c
case value:
    return ;
    break;
```

`case` ships with its own `break;` and a tabstop for the label value. The `>` child operator nests a statement, here a `ret`, between the label and the `break;`.

### `def`

`#define` constant.

```sh
codemmet expand --lang c "def:MAX_LEN"
```

```c
#define MAX_LEN 
```

The explicit argument `:MAX_LEN` becomes the macro name, and a tabstop is left after it for the value.

### `default`

Default label with break.

```sh
codemmet expand --lang c "default"
```

```c
default:
    
    break;
```

```sh
codemmet expand --lang c "default>ret"
```

```c
default:
    return ;
    break;
```

Like `case`, `default` provides its own `break;`. The `>` child operator places a nested statement, here `ret`, above it.

### `doc`

Doc comment line.

```sh
codemmet expand --lang c "doc"
```

```c
/**  */
```

`doc` takes no arguments and emits a single-line `/** */` comment with a tabstop for the text.

### `dowhile`

Do-while loop.

```sh
codemmet expand --lang c "dowhile"
```

```c
do {
    
} while (condition);
```

```sh
codemmet expand --lang c "dowhile>ret"
```

```c
do {
    return ;
} while (condition);
```

`dowhile` emits a full `do { } while` with tabstops for the body and condition. The `>` child operator fills the body, here with a `ret`.

### `el`

Else block.

```sh
codemmet expand --lang c "el"
```

```c
else {
    
}
```

```sh
codemmet expand --lang c "el>ret"
```

```c
else {
    return ;
}
```

`el` expands to a bare `else` block. The `>` child operator nests a statement in its body, here a `ret`.

### `elif`

Else-if block.

```sh
codemmet expand --lang c "elif"
```

```c
else if (condition) {
    
}
```

```sh
codemmet expand --lang c "elif>ret"
```

```c
else if (condition) {
    return ;
}
```

`elif` emits an `else if` with a tabstop for the condition. The `>` child operator fills the body, here with a `ret`.

### `enum`

Typedef'd enum definition.

```sh
codemmet expand --lang c "enum:Color"
```

```c
typedef enum {
    
} Color;
```

```sh
codemmet expand --lang c "enum:Color>doc"
```

```c
typedef enum {
    /**  */
} Color;
```

The explicit argument `:Color` names the type. The `>` child operator nests body content, here a `doc` comment line among the enumerators.

### `fn`

Function definition.

```sh
codemmet expand --lang c "fn:sum"
```

```c
void sum() {
    
}
```

```sh
codemmet expand --lang c "fn:sum>ret"
```

```c
void sum() {
    return ;
}
```

The explicit argument `:sum` names the function, with tabstops for the return type and parameters. The `>` child operator drops statements into the body, here a `ret`.

### `fori`

For loop over integer range.

```sh
codemmet expand --lang c "fori10"
```

```c
for (int i = 0; i < 10; i++) {
    
}
```

```sh
codemmet expand --lang c 'fori10>prf:"%d\n",i'
```

```c
for (int i = 0; i < 10; i++) {
    printf("%d\n", i);
}
```

The glued argument `10` sets the loop bound while the counter defaults to `i`. The `>` child operator nests the body, here a `prf` call formatting the counter.

### `free`

Free a pointer and null it.

```sh
codemmet expand --lang c "free:buf"
```

```c
free(buf);
buf = NULL;
```

The explicit argument `:buf` names the pointer, which is both freed and set to `NULL` on the following line.

### `guard`

Include guard.

```sh
codemmet expand --lang c "guard:UTIL_H"
```

```c
#ifndef UTIL_H
#define UTIL_H



#endif /* UTIL_H */
```

The explicit argument `:UTIL_H` becomes the guard macro, repeated across the `#ifndef`, `#define`, and closing `#endif` comment, with a tabstop for the guarded body.

### `if`

If block.

```sh
codemmet expand --lang c "if"
```

```c
if (condition) {
    
}
```

```sh
codemmet expand --lang c "if>ret"
```

```c
if (condition) {
    return ;
}
```

`if` emits a conditional with a tabstop for the test. The `>` child operator fills the body, here with a `ret`.

### `inc`

`#include` a system header.

```sh
codemmet expand --lang c "inc:stdio.h"
```

```c
#include <stdio.h>
```

The explicit argument `:stdio.h` is placed inside angle brackets for a system include.

### `incl`

`#include` a local header.

```sh
codemmet expand --lang c "incl:util.h"
```

```c
#include "util.h"
```

The explicit argument `:util.h` is placed inside double quotes for a local include.

### `main`

Main entry point.

```sh
codemmet expand --lang c "main"
```

```c
int main(void) {
    
    return 0;
}
```

```sh
codemmet expand --lang c "main>fori10"
```

```c
int main(void) {
    for (int i = 0; i < 10; i++) {
        
    }
    return 0;
}
```

`main` includes its own trailing `return 0;`. The `>` child operator nests statements above it, here a `fori10` counted loop.

### `malloc`

Malloc a typed buffer.

```sh
codemmet expand --lang c "malloc:int,buf,n"
```

```c
int *buf = malloc(n * sizeof(int));
```

The three comma-separated arguments set the element type (`int`), the pointer name (`buf`), and the element count (`n`), which is multiplied by `sizeof` the type.

### `pr`

Printf a line to stdout.

```sh
codemmet expand --lang c "pr"
```

```c
printf("\n");
```

`pr` takes no arguments; it prints a literal trailing newline and leaves a tabstop for the message text.

### `prf`

Printf with a format string and arguments.

```sh
codemmet expand --lang c 'prf:"%d\n",i'
```

```c
printf("%d\n", i);
```

The first argument is the format string (verbatim, including your own `\n`) and the rest are the printf arguments, here the single value `i`.

### `ret`

Return statement.

```sh
codemmet expand --lang c "ret"
```

```c
return ;
```

`ret` takes no arguments and leaves a tabstop for the returned expression.

### `sizeof`

Sizeof expression.

```sh
codemmet expand --lang c "sizeof"
```

```c
sizeof()
```

`sizeof` takes no arguments and leaves a tabstop inside the parentheses for the type or expression.

### `struct`

Typedef'd struct definition.

```sh
codemmet expand --lang c "struct:Point"
```

```c
typedef struct {
    
} Point;
```

```sh
codemmet expand --lang c "struct:Point>doc"
```

```c
typedef struct {
    /**  */
} Point;
```

The explicit argument `:Point` names the type. The `>` child operator nests body content, here a `doc` comment among the fields.

### `switch`

Switch statement.

```sh
codemmet expand --lang c "switch:cmd"
```

```c
switch (cmd) {
    
}
```

```sh
codemmet expand --lang c "switch:cmd>case+case+default"
```

```c
switch (cmd) {
    case value:
        
        break;
    case value:
        
        break;
    default:
        
        break;
}
```

The explicit argument `:cmd` is the tested expression. The `>` child operator nests the body, and the `+` sibling operator lays out two `case` blocks and a `default` at the same level.

### `td`

Typedef declaration.

```sh
codemmet expand --lang c "td"
```

```c
typedef int name_t;
```

`td` takes no arguments and leaves tabstops for the underlying type (defaulting to `int`) and the new type name.

### `todo`

TODO comment.

```sh
codemmet expand --lang c "todo"
```

```c
// TODO: 
```

`todo` takes no arguments and emits a `// TODO:` line with a tabstop for the note.

### `while`

While loop.

```sh
codemmet expand --lang c "while"
```

```c
while (condition) {
    
}
```

```sh
codemmet expand --lang c "while>ret"
```

```c
while (condition) {
    return ;
}
```

`while` emits a loop with a tabstop for the condition. The `>` child operator fills the body, here with a `ret`.

## See also

- [abbreviation syntax](../syntax.md)
- [language reference](../languages.md)
- [examples overview](README.md)
