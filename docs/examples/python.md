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

## Trigger reference

One entry per Python trigger, in alphabetical order. Block and named triggers show a simple form and a composed form; leaf triggers show a single form.

### `adef`

Async function definition.

```sh
codemmet expand --lang python "adef:fetch"
```

```python
async def fetch():
    
```

The glued argument names the coroutine. Composed with a child:

```sh
codemmet expand --lang python "adef:fetch>ret"
```

```python
async def fetch():
    return 
```

The `>` child operator nests a `return` statement inside the body.

### `aeq`

Equality assertion.

```sh
codemmet expand --lang python "aeq"
```

```python
assert actual == 
```

Emits an `assert actual == ` line with tabstops for the actual and expected values.

### `arm`

Case clause of a `match` statement.

```sh
codemmet expand --lang python "arm"
```

```python
case pattern:
    
```

A single `case` clause on its own. Nested inside a `match`:

```sh
codemmet expand --lang python "match:cmd>arm"
```

```python
match cmd:
    case pattern:
        
```

The `>` child operator places the `case` clause inside the `match` on subject `cmd`.

### `class`

Class definition.

```sh
codemmet expand --lang python "class:Point"
```

```python
class Point:
    
```

The glued argument names the class. Composed with members:

```sh
codemmet expand --lang python "class:Point>init+str"
```

```python
class Point:
    def __init__(self):
        
    def __str__(self) -> str:
        return f""
```

The `>` child operator nests members, and the `+` sibling operator joins the `__init__` and `__str__` dunders.

### `dc`

Dict comprehension.

```sh
codemmet expand --lang python "dc"
```

```python
{key: value for item in iterable}
```

Produces a dict-comprehension skeleton with tabstops for the key, value, loop variable, and iterable.

### `def`

Function definition.

```sh
codemmet expand --lang python "def:load"
```

```python
def load():
    
```

The glued argument names the function. Composed with a child:

```sh
codemmet expand --lang python "def:load>ret"
```

```python
def load():
    return 
```

The `>` child operator nests a `return` statement inside the body.

### `doc`

Docstring.

```sh
codemmet expand --lang python "doc"
```

```python
""""""
```

Emits a triple-quoted docstring with the cursor between the quotes.

### `el`

Else block.

```sh
codemmet expand --lang python "el"
```

```python
else:
    
```

An `else:` clause on its own. Composed after an `if`:

```sh
codemmet expand --lang python "if>pr^el>pr"
```

```python
if condition:
    print()
else:
    print()
```

The `^` climb-up operator lifts `el` back out to the `if`'s level so it reads as its `else` branch.

### `elif`

Elif block.

```sh
codemmet expand --lang python "elif"
```

```python
elif condition:
    
```

An `elif condition:` clause on its own. Composed after an `if`:

```sh
codemmet expand --lang python "if>pr^elif>pr"
```

```python
if condition:
    print()
elif condition:
    print()
```

The `^` climb-up operator lifts `elif` back out to the `if`'s level so it chains onto it.

### `exc`

Except clause.

```sh
codemmet expand --lang python "exc"
```

```python
except Exception as exc:
    
```

An `except Exception as exc:` handler on its own. Composed with a body:

```sh
codemmet expand --lang python "exc>pr"
```

```python
except Exception as exc:
    print()
```

The `>` child operator nests a `print()` call inside the handler body.

### `fin`

Finally clause.

```sh
codemmet expand --lang python "fin"
```

```python
finally:
    
```

A `finally:` clause on its own. Composed with a body:

```sh
codemmet expand --lang python "fin>pr"
```

```python
finally:
    print()
```

The `>` child operator nests a `print()` call inside the clause body.

### `fore`

For-each loop over an iterable.

```sh
codemmet expand --lang python "fore:item"
```

```python
for item in iterable:
    
```

The glued argument names the loop variable. Composed with a body:

```sh
codemmet expand --lang python "fore:line>pr"
```

```python
for line in iterable:
    print()
```

The `>` child operator nests a `print()` call inside the loop.

### `fori`

For loop over an integer range.

```sh
codemmet expand --lang python "fori10"
```

```python
for i in range(10):
    
```

The glued number becomes the `range()` bound. Composed with a body:

```sh
codemmet expand --lang python "fori10>pr"
```

```python
for i in range(10):
    print()
```

The `>` child operator nests a `print()` call inside the loop.

### `from`

From-import statement.

```sh
codemmet expand --lang python "from:pathlib"
```

```python
from pathlib import 
```

The glued argument names the module; a tabstop follows `import` for the names to bring in.

### `if`

If block.

```sh
codemmet expand --lang python "if"
```

```python
if condition:
    
```

An `if condition:` header on its own. Composed with a body:

```sh
codemmet expand --lang python "if>pr"
```

```python
if condition:
    print()
```

The `>` child operator nests a `print()` call inside the branch.

### `ifmain`

`__main__` guard block.

```sh
codemmet expand --lang python "ifmain"
```

```python
if __name__ == "__main__":
    
```

The `if __name__ == "__main__":` guard on its own. Composed with a body:

```sh
codemmet expand --lang python "ifmain>pr"
```

```python
if __name__ == "__main__":
    print()
```

The `>` child operator nests a `print()` call inside the guard.

### `imp`

Import statement.

```sh
codemmet expand --lang python "imp:os"
```

```python
import os
```

The glued argument names the module to import.

### `init`

`__init__` method (dunder).

```sh
codemmet expand --lang python "init"
```

```python
def __init__(self):
    
```

The constructor dunder on its own. Composed inside a class:

```sh
codemmet expand --lang python "class:Point>init"
```

```python
class Point:
    def __init__(self):
        
```

The `>` child operator nests the `__init__` method inside the class body.

### `lam`

Lambda expression.

```sh
codemmet expand --lang python "lam"
```

```python
lambda : 
```

Emits a `lambda` with tabstops for the parameters and the body expression.

### `lc`

List comprehension.

```sh
codemmet expand --lang python "lc"
```

```python
[expr for item in iterable]
```

Produces a list-comprehension skeleton with tabstops for the expression, loop variable, and iterable.

### `main`

Main function with `__main__` guard.

```sh
codemmet expand --lang python "main"
```

```python
def main():
    


if __name__ == "__main__":
    main()
```

The `main()` function together with the guard that calls it. Composed with a body:

```sh
codemmet expand --lang python "main>pr"
```

```python
def main():
    print()


if __name__ == "__main__":
    main()
```

The `>` child operator nests a `print()` call inside `main()`; the `if __name__ == "__main__":` guard follows.

### `match`

Match statement.

```sh
codemmet expand --lang python "match:cmd"
```

```python
match cmd:
    
```

The glued argument names the subject. Composed with cases:

```sh
codemmet expand --lang python "match:cmd>arm*2"
```

```python
match cmd:
    case pattern:
        
    case pattern:
        
```

The `>` child operator nests the cases, and the `*` repeat operator produces the `case` clause twice.

### `pr`

Print call.

```sh
codemmet expand --lang python "pr"
```

```python
print()
```

Emits a `print()` call with the cursor between the parentheses.

### `prd`

Debug print using the f-string `=` specifier.

```sh
codemmet expand --lang python "prd"
```

```python
print(f"{=}")
```

Emits a `print(f"{=}")` where the `=` specifier echoes both the expression and its value.

### `prop`

Property getter.

```sh
codemmet expand --lang python "prop:balance"
```

```python
@property
def balance(self):
    return self._balance
```

The glued argument names the property and is mirrored into `self._balance`. Composed inside a class:

```sh
codemmet expand --lang python "class:Account>prop:balance"
```

```python
class Account:
    @property
    def balance(self):
        return self._balance
```

The `>` child operator nests the `@property` getter inside the class body.

### `raise`

Raise an exception.

```sh
codemmet expand --lang python "raise"
```

```python
raise ValueError()
```

Emits a `raise ValueError()` with tabstops for the exception type and message.

### `repr`

`__repr__` dunder method.

```sh
codemmet expand --lang python "repr"
```

```python
def __repr__(self) -> str:
    return f"{type(self).__name__}()"
```

The dunder on its own, pre-filled with the class name. Composed inside a class:

```sh
codemmet expand --lang python "class:Point>repr"
```

```python
class Point:
    def __repr__(self) -> str:
        return f"{type(self).__name__}()"
```

The `>` child operator nests the `__repr__` method inside the class body.

### `ret`

Return statement.

```sh
codemmet expand --lang python "ret"
```

```python
return 
```

Emits a `return ` with a tabstop for the value.

### `str`

`__str__` dunder method.

```sh
codemmet expand --lang python "str"
```

```python
def __str__(self) -> str:
    return f""
```

The dunder on its own. Composed inside a class:

```sh
codemmet expand --lang python "class:Point>str"
```

```python
class Point:
    def __str__(self) -> str:
        return f""
```

The `>` child operator nests the `__str__` method inside the class body.

### `test`

Test function (pytest style).

```sh
codemmet expand --lang python "test:parses"
```

```python
def test_parses():
    
```

The glued argument is appended to the `test_` prefix. Composed with a body:

```sh
codemmet expand --lang python "test:parses>aeq"
```

```python
def test_parses():
    assert actual == 
```

The `>` child operator nests an equality assertion inside the test.

### `todo`

TODO comment.

```sh
codemmet expand --lang python "todo"
```

```python
# TODO: 
```

Emits a `# TODO: ` comment with the cursor after the colon.

### `try`

Try/except block.

```sh
codemmet expand --lang python "try"
```

```python
try:
    
except Exception as exc:
    
```

The `try` header with a matching `except Exception as exc:` handler. Composed inside a function:

```sh
codemmet expand --lang python "def:run>try"
```

```python
def run():
    try:
        
    except Exception as exc:
        
```

The `>` child operator nests the whole `try`/`except` block inside `run()`.

### `while`

While loop.

```sh
codemmet expand --lang python "while"
```

```python
while condition:
    
```

A `while condition:` header on its own. Composed with a body:

```sh
codemmet expand --lang python "while>pr"
```

```python
while condition:
    print()
```

The `>` child operator nests a `print()` call inside the loop.

### `with`

With statement (context manager).

```sh
codemmet expand --lang python "with"
```

```python
with expr as name:
    
```

A `with expr as name:` header on its own. Composed with a body:

```sh
codemmet expand --lang python "with>pr"
```

```python
with expr as name:
    print()
```

The `>` child operator nests a `print()` call inside the context manager block.

## See also

- [abbreviation syntax](../syntax.md)
- [language reference](../languages.md)
- [examples overview](README.md)
