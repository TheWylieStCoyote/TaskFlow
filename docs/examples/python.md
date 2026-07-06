# Python examples

The Python pack has language id `python`, so `--lang` matches it by that id, by the `py` file extension, or by the `python` LSP language id; expansions indent with 4 spaces. See the [language reference](../languages.md) for the complete trigger list.

> Examples use the default `strip` mode, which renders plain code and leaves an empty spot wherever a tabstop will be — those are the places you fill in. Add `--tabstops keep` to see them as `$1` / `${1:placeholder}` / `$0` markers. See the [examples overview](README.md) for more.

## A class with a constructor, a property, and a dunder

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

The `>` child operator nests the members inside the class, and the `+` sibling operator joins them so `init`, `prop:balance`, and `str` follow one another. The `prop` trigger takes a glued name argument and mirrors that name into `self._balance` in the return line.

## A class with two properties

```sh
codemmet expand --lang python "class:Temp>prop:celsius+prop:fahrenheit"
```

```python
class Temp:
    @property
    def celsius(self):
        return self._celsius
    @property
    def fahrenheit(self):
        return self._fahrenheit
```

Each `prop:<name>` emits a `@property` getter, and the glued name is mirrored into the `self._<name>` it returns. The `+` sibling operator keeps both properties at the same level inside the class.

## A main entry point with the __main__ guard

```sh
codemmet expand --lang python "main>pr"
```

```python
def main():
    print()


if __name__ == "__main__":
    main()
```

The `main` trigger emits a `main()` function together with the `if __name__ == "__main__":` guard that calls it. The `>` child operator nests `pr` (a `print()` call) inside the function body.

## Structural pattern matching

```sh
codemmet expand --lang python "def:handle>match:cmd>arm*2"
```

```python
def handle():
    match cmd:
        case pattern:
            
        case pattern:
            
```

The `match:cmd` trigger opens a `match` on the glued subject `cmd`, and the `>` child operator nests the cases inside it. The `*` repeat operator in `arm*2` produces the `case` clause twice.

## Reading a file line by line

```sh
codemmet expand --lang python "def:read>with>fore:line>pr"
```

```python
def read():
    with expr as name:
        for line in iterable:
            print()
```

Each `>` child operator nests the next construct one level deeper: a `with` statement, then a `for` loop whose glued argument names the loop variable `line`, then a `print()`. The `fore` trigger produces the `for ... in ...:` header.

## A nested loop with error handling

```sh
codemmet expand --lang python "def:sync>fore:item>try"
```

```python
def sync():
    for item in iterable:
        try:
            
        except Exception as exc:
            
```

The `fore:item` trigger names the loop variable `item` through its glued argument, and the `>` child operator nests a `try`/`except` block inside the loop. The `try` trigger fills in the `except Exception as exc:` handler.

## See also

- [abbreviation syntax](../syntax.md)
- [language reference](../languages.md)
- [examples overview](README.md)
