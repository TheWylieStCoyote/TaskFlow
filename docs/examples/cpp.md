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

## See also

- [abbreviation syntax](../syntax.md)
- [language reference](../languages.md)
- [examples overview](README.md)
