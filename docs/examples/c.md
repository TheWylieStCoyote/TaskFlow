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

## See also

- [abbreviation syntax](../syntax.md)
- [language reference](../languages.md)
- [examples overview](README.md)
