# C++ examples

The C++ pack has language id `cpp` and matches on the `cpp`, `cc`, `cxx`, `hpp`, `hh`, and `hxx` file extensions or the LSP id `cpp`. It indents with 4 spaces by default and inherits the [C](c.md) pack, starting from C's triggers and overriding or adding on top; see the [language reference](../languages.md) for details.

> Examples use the default `strip` mode, which renders plain code and leaves an empty spot wherever a tabstop will be — those are the places you fill in. Add `--tabstops keep` to see them as `$1` / `${1:placeholder}` / `$0` markers. See the [examples overview](README.md) for more.

## A class with a method

```sh
codemmet expand --lang cpp "class:Widget>fn:area>ret"
```

```cpp
class Widget {
public:
    void area() {
        return ;
    }
};
```

The `class` trigger names the type with its explicit argument `:Widget` and opens a `public:` section. The `>` child operator nests the `fn` method inside the class, and nests `ret` inside the method body.

## A namespace holding a class

```sh
codemmet expand --lang cpp "ns:geo>class:Point>fn:norm"
```

```cpp
namespace geo {

class Point {
public:
    void norm() {
        
    }
};

}  // namespace geo
```

The `ns` trigger wraps its contents in a `namespace`, using the explicit argument `:geo` for the name and echoing it in the closing comment. Two `>` child operators nest the `class` inside the namespace and the `fn` method inside the class.

## A vector and a range-based for loop

```sh
codemmet expand --lang cpp "fn:process>vec>fore>cout"
```

```cpp
void process() {
    std::vector<int> values;
    for (const auto& item : container) {
        std::cout <<  << '\n';
    }
}
```

`vec`, `fore`, and `cout` are C++ additions layered over the inherited C pack: `vec` declares a `std::vector`, `fore` produces the range-based `for` form, and `cout` writes a stream statement. The `>` child operator nests the loop inside the function and the `cout` call inside the loop.

## A function template

```sh
codemmet expand --lang cpp "tpl>fn:max>ret"
```

```cpp
template <typename T>
void max() {
    return ;
}
```

The `tpl` trigger emits a `template <typename T>` header, and the `>` child operator attaches the `fn` function beneath it with `ret` in its body.

## A unique pointer

```sh
codemmet expand --lang cpp "fn:make>uptr+ret"
```

```cpp
void make() {
    auto ptr = std::make_unique<Type>();
    return ;
}
```

The `uptr` trigger creates a `std::make_unique` binding. It and `ret` are joined by the `+` sibling operator so they sit at the same level, both nested in the `fn` body by the `>` child operator.

## A map you iterate

```sh
codemmet expand --lang cpp "fn:count>map>fore>cout"
```

```cpp
void count() {
    std::unordered_map<std::string, int> map;
    for (const auto& item : container) {
        std::cout <<  << '\n';
    }
}
```

The `map` trigger declares a `std::unordered_map`, and `fore` gives the range-based `for` loop that walks it. The `>` child operator nests the loop after the map declaration and places the `cout` statement inside the loop body.

## Trigger reference

One entry per C++ trigger, in alphabetical order. The C++ pack inherits the [C](c.md) pack, so many triggers behave exactly as they do in C; entries note where a trigger overrides a C form or is a C++-only addition. Block and named triggers show a simple form and a composed form that nests a child with the `>` operator.

### `aeq`

Equality assertion.

```sh
codemmet expand --lang cpp "aeq"
```

```cpp
assert(actual == );
```

Inherited unchanged from C. Fills in the two operands of an `assert` equality check.

### `assert`

Assert statement.

```sh
codemmet expand --lang cpp "assert"
```

```cpp
assert();
```

Inherited unchanged from C. Wraps a boolean expression in an `assert` call.

### `case`

Case label with break.

```sh
codemmet expand --lang cpp "case"
```

```cpp
case value:
    
    break;
```

```sh
codemmet expand --lang cpp "case>pr"
```

```cpp
case value:
    printf("\n");
    break;
```

Inherited unchanged from C. The `>` child operator nests statements between the label and its trailing `break;`.

### `class`

Class definition.

```sh
codemmet expand --lang cpp "class:Widget"
```

```cpp
class Widget {
public:
    
};
```

```sh
codemmet expand --lang cpp "class:Widget>fn:area"
```

```cpp
class Widget {
public:
    void area() {
        
    }
};
```

A C++-only addition with no C counterpart. It names the type from the explicit `:Widget` argument, opens a `public:` section, and the `>` child operator nests members such as the `fn` method inside.

### `cout`

std::cout line.

```sh
codemmet expand --lang cpp "cout"
```

```cpp
std::cout <<  << '\n';
```

A C++-only addition. Writes a value to `std::cout` followed by a newline.

### `def`

#define constant.

```sh
codemmet expand --lang cpp "def"
```

```cpp
#define NAME 
```

Inherited unchanged from C. Defines an object-like macro with a name and replacement value.

### `default`

Default label with break.

```sh
codemmet expand --lang cpp "default"
```

```cpp
default:
    
    break;
```

```sh
codemmet expand --lang cpp "default>pr"
```

```cpp
default:
    printf("\n");
    break;
```

Inherited unchanged from C. The `>` child operator nests statements between the `default:` label and its trailing `break;`.

### `doc`

Doc comment line.

```sh
codemmet expand --lang cpp "doc"
```

```cpp
/**  */
```

Inherited unchanged from C. Produces a single-line `/** ... */` documentation comment.

### `dowhile`

Do-while loop.

```sh
codemmet expand --lang cpp "dowhile"
```

```cpp
do {
    
} while (condition);
```

```sh
codemmet expand --lang cpp "dowhile>cout"
```

```cpp
do {
    std::cout <<  << '\n';
} while (condition);
```

Inherited unchanged from C. The `>` child operator nests statements inside the loop body that runs before the `while` condition is tested.

### `el`

Else block.

```sh
codemmet expand --lang cpp "el"
```

```cpp
else {
    
}
```

```sh
codemmet expand --lang cpp "el>ret"
```

```cpp
else {
    return ;
}
```

Inherited unchanged from C. The `>` child operator nests statements inside the `else` body.

### `elif`

Else-if block.

```sh
codemmet expand --lang cpp "elif"
```

```cpp
else if (condition) {
    
}
```

```sh
codemmet expand --lang cpp "elif>ret"
```

```cpp
else if (condition) {
    return ;
}
```

Inherited unchanged from C. Produces an `else if` chain link; the `>` child operator nests statements in its body.

### `enum`

Scoped enum.

```sh
codemmet expand --lang cpp "enum:Color"
```

```cpp
enum class Color {
    
};
```

```sh
codemmet expand --lang cpp "enum:Color>doc"
```

```cpp
enum class Color {
    /**  */
};
```

Overrides C's `typedef enum { ... }` form with a C++ scoped `enum class`. The name comes from the explicit `:Color` argument, and the `>` child operator nests content inside the enumerator list.

### `fn`

Function definition.

```sh
codemmet expand --lang cpp "fn:area"
```

```cpp
void area() {
    
}
```

```sh
codemmet expand --lang cpp "fn:process>vec"
```

```cpp
void process() {
    std::vector<int> values;
}
```

Inherited from C. It names the function from the explicit `:area` argument, and the `>` child operator nests statements inside the body.

### `fore`

Range-based for loop.

```sh
codemmet expand --lang cpp "fore"
```

```cpp
for (const auto& item : container) {
    
}
```

```sh
codemmet expand --lang cpp "fore>cout"
```

```cpp
for (const auto& item : container) {
    std::cout <<  << '\n';
}
```

A C++-only addition (C has only the counting `fori`). Produces the range-based `for` form; the `>` child operator nests statements inside the loop body.

### `fori`

For loop over an integer range.

```sh
codemmet expand --lang cpp "fori10"
```

```cpp
for (int i = 0; i < 10; i++) {
    
}
```

```sh
codemmet expand --lang cpp "fori10>pr"
```

```cpp
for (int i = 0; i < 10; i++) {
    printf("\n");
}
```

Inherited unchanged from C. The `10` suffix sets the loop bound, and the `>` child operator nests statements inside the loop body.

### `free`

Free a pointer and null it.

```sh
codemmet expand --lang cpp "free"
```

```cpp
free(ptr);
ptr = NULL;
```

Inherited unchanged from C. Releases a heap allocation and clears the pointer to guard against reuse.

### `guard`

Include guard.

```sh
codemmet expand --lang cpp "guard"
```

```cpp
#ifndef HEADER_H
#define HEADER_H



#endif /* HEADER_H */
```

Inherited unchanged from C. Wraps a header's contents in an `#ifndef`/`#define`/`#endif` include guard.

### `if`

If block.

```sh
codemmet expand --lang cpp "if"
```

```cpp
if (condition) {
    
}
```

```sh
codemmet expand --lang cpp "if>ret"
```

```cpp
if (condition) {
    return ;
}
```

Inherited unchanged from C. The `>` child operator nests statements inside the `if` body.

### `inc`

#include a system header.

```sh
codemmet expand --lang cpp "inc:vector"
```

```cpp
#include <vector>
```

Inherited from C. Emits an angle-bracket `#include`, taking the header name from the explicit `:vector` argument.

### `incl`

#include a local header.

```sh
codemmet expand --lang cpp "incl"
```

```cpp
#include "header.h"
```

Inherited unchanged from C. Emits a quoted `#include` for a project-local header.

### `lam`

Lambda expression.

```sh
codemmet expand --lang cpp "lam"
```

```cpp
[]() {  }
```

A C++-only addition. Produces a lambda with capture list, parameter list, and body.

### `main`

Main entry point.

```sh
codemmet expand --lang cpp "main"
```

```cpp
int main(void) {
    
    return 0;
}
```

```sh
codemmet expand --lang cpp "main>ret"
```

```cpp
int main(void) {
    return ;
    return 0;
}
```

Inherited from C. Produces a `main` function with a trailing `return 0;`; the `>` child operator nests statements ahead of it.

### `malloc`

malloc a typed buffer.

```sh
codemmet expand --lang cpp "malloc"
```

```cpp
int *ptr = malloc(1 * sizeof(int));
```

Inherited unchanged from C. Allocates a typed buffer sized by an element count times `sizeof`.

### `map`

std::unordered_map variable.

```sh
codemmet expand --lang cpp "map"
```

```cpp
std::unordered_map<std::string, int> map;
```

A C++-only addition. Declares a `std::unordered_map` variable with key and value types.

### `ns`

Namespace block.

```sh
codemmet expand --lang cpp "ns:geo"
```

```cpp
namespace geo {



}  // namespace geo
```

```sh
codemmet expand --lang cpp "ns:geo>class:Point"
```

```cpp
namespace geo {

class Point {
public:
    
};

}  // namespace geo
```

A C++-only addition. It names the namespace from the explicit `:geo` argument, echoes it in the closing comment, and the `>` child operator nests declarations inside.

### `pr`

printf a line to stdout.

```sh
codemmet expand --lang cpp "pr"
```

```cpp
printf("\n");
```

Inherited unchanged from C. Prints a plain newline-terminated line.

### `prf`

printf with a format string and arguments.

```sh
codemmet expand --lang cpp "prf"
```

```cpp
printf("%d\n", value);
```

Inherited unchanged from C. Prints a formatted line with a format string and argument list.

### `ret`

Return statement.

```sh
codemmet expand --lang cpp "ret"
```

```cpp
return ;
```

Inherited unchanged from C. Returns an optional value from the enclosing function.

### `sizeof`

sizeof expression.

```sh
codemmet expand --lang cpp "sizeof"
```

```cpp
sizeof()
```

Inherited unchanged from C. Produces a `sizeof` expression around a type or value.

### `sptr`

std::make_shared binding.

```sh
codemmet expand --lang cpp "sptr"
```

```cpp
auto ptr = std::make_shared<Type>();
```

A C++-only addition. Binds an `auto` variable to a `std::make_shared` allocation.

### `struct`

Struct definition.

```sh
codemmet expand --lang cpp "struct:Point"
```

```cpp
struct Point {
    
};
```

```sh
codemmet expand --lang cpp "struct:Point>fn:norm"
```

```cpp
struct Point {
    void norm() {
        
    }
};
```

Overrides C's `typedef struct { ... }` form with a plain C++ `struct Name { ... }`. The name comes from the explicit `:Point` argument, and the `>` child operator nests members inside.

### `switch`

Switch statement.

```sh
codemmet expand --lang cpp "switch"
```

```cpp
switch (value) {
    
}
```

```sh
codemmet expand --lang cpp "switch>case"
```

```cpp
switch (value) {
    case value:
        
        break;
}
```

Inherited unchanged from C. The `>` child operator nests `case` and `default` labels inside the switch body.

### `td`

typedef declaration.

```sh
codemmet expand --lang cpp "td"
```

```cpp
typedef int name_t;
```

Inherited unchanged from C. Declares a `typedef` alias for an existing type.

### `todo`

TODO comment.

```sh
codemmet expand --lang cpp "todo"
```

```cpp
// TODO: 
```

Inherited unchanged from C. Emits a `// TODO:` marker comment.

### `tpl`

Template parameter list (prefix for a declaration).

```sh
codemmet expand --lang cpp "tpl"
```

```cpp
template <typename T>
```

```sh
codemmet expand --lang cpp "tpl>fn:max"
```

```cpp
template <typename T>
void max() {
    
}
```

A C++-only addition. It emits a `template <typename T>` header, and the `>` child operator attaches the templated declaration, such as the `fn` function, beneath it.

### `try`

Try/catch block.

```sh
codemmet expand --lang cpp "try"
```

```cpp
try {
    
} catch (const std::exception& e) {
    
}
```

```sh
codemmet expand --lang cpp "try>cout"
```

```cpp
try {
    std::cout <<  << '\n';
} catch (const std::exception& e) {

}
```

A C++-only addition. Produces a `try` block with a `catch (const std::exception& e)` handler; the `>` child operator nests statements inside the `try` body.

### `uptr`

std::make_unique binding.

```sh
codemmet expand --lang cpp "uptr"
```

```cpp
auto ptr = std::make_unique<Type>();
```

A C++-only addition. Binds an `auto` variable to a `std::make_unique` allocation.

### `using`

using type alias.

```sh
codemmet expand --lang cpp "using"
```

```cpp
using Alias = ;
```

A C++-only addition (the modern alternative to `td`). Declares a `using` type alias.

### `vec`

std::vector variable.

```sh
codemmet expand --lang cpp "vec"
```

```cpp
std::vector<int> values;
```

A C++-only addition. Declares a `std::vector` variable with its element type.

### `while`

While loop.

```sh
codemmet expand --lang cpp "while"
```

```cpp
while (condition) {
    
}
```

```sh
codemmet expand --lang cpp "while>cout"
```

```cpp
while (condition) {
    std::cout <<  << '\n';
}
```

Inherited unchanged from C. The `>` child operator nests statements inside the loop body.

## See also

- [abbreviation syntax](../syntax.md)
- [language reference](../languages.md)
- [examples overview](README.md)
