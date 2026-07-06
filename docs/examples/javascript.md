# JavaScript examples

The `javascript` language handles files with the `js`, `mjs`, `cjs`, and `jsx` extensions, and matches the LSP ids `javascript` and `javascriptreact`. Pass `--lang javascript`, or let codemmet match on any of those ids or extensions. Its default indent is 2 spaces. See the [language reference](../languages.md) for the full list of triggers.

> Examples use the default `strip` mode, which renders plain code and leaves an empty spot wherever a tabstop will be — those are the places you fill in. Add `--tabstops keep` to see them as `$1` / `${1:placeholder}` / `$0` markers. See the [examples overview](README.md) for more.

## A class with a constructor and a method

```sh
codemmet expand --lang javascript "class:Store>ctor+fn:add"
```

```javascript
class Store {
  constructor() {
    
  }
  add() {
    
  }
}
```

The `add()` method has no `function` keyword: a `fn` composed inside a class picks the method form rather than the standalone function form. codemmet reads that context from the composition itself, so the same trigger adapts to where you place it.

## An async function with try/await

```sh
codemmet expand --lang javascript "afn:load>try>aw"
```

```javascript
async function load() {
  try {
    await ;
  } catch (err) {

  }
}
```

The `afn` trigger takes `load` as a glued argument to name the function, `try` nests a try/catch, and `aw` drops an `await` expression inside it. Fill the tabstop after `await` with the promise you want to resolve.

## A getter and setter pair

```sh
codemmet expand --lang javascript "class:Box>get+set"
```

```javascript
class Box {
  get value() {
    return this._value;
  }
  set value(value) {
    this._value = value;
  }
}
```

The `+` sibling operator places the `get` and `set` accessors side by side inside the class body. Both default to a `value` property backed by `this._value`.

## A Promise

```sh
codemmet expand --lang javascript "fn:delay>prom"
```

```javascript
function delay() {
  new Promise((resolve, reject) => {
    
  })
}
```

The `prom` trigger expands to a `new Promise` with `resolve` and `reject` parameters. The tabstop inside the executor is where you place the work that settles the promise.

## Routing with a switch

```sh
codemmet expand --lang javascript "fn:route>switch:method>case+case+default"
```

```javascript
function route() {
  switch (method) {
    case value:
      
      break;
    case value:
      
      break;
    default:
      
  }
}
```

The `switch` trigger takes `method` as a glued argument for the discriminant, and each child `case` (plus the trailing `default`) becomes a branch. The two `case` siblings repeat the branch shape, each with its own body tabstop and `break`.

## Imports and exports

```sh
codemmet expand --lang javascript "imp+impd+expd"
```

```javascript
import name from "module";
import {  } from "module";
export default ;
```

Three sibling statements: `imp` for a default import, `impd` for a named (destructured) import, and `expd` for a default export. Fill in the names, the destructured bindings, and the exported value at the tabstops.

## Trigger reference

One entry per JavaScript trigger, in alphabetical order. Each shows the trigger on its own; block and named triggers also show a composed form so you can see how they nest.

### `aeq`

An equality expectation.

```sh
codemmet expand --lang javascript "aeq"
```

```javascript
expect(actual).toBe();
```

`aeq` expands to an `expect(...).toBe(...)` assertion. Fill the actual value and the expected value at the two tabstops.

### `afn`

An async function declaration.

```sh
codemmet expand --lang javascript "afn:load"
```

```javascript
async function load() {
  
}
```

The glued argument names the function, so `afn:load` produces `async function load()`. The body tabstop is where the awaited work goes.

```sh
codemmet expand --lang javascript "afn:load>try"
```

```javascript
async function load() {
  try {
    
  } catch (err) {
    
  }
}
```

Composing `>try` nests a try/catch inside the async function body, the usual shape for awaiting a call that may reject.

### `arrow`

An arrow function bound to a const.

```sh
codemmet expand --lang javascript "arrow:double"
```

```javascript
const double = () => {
  
};
```

The argument names the binding, so `arrow:double` declares `const double = () => { … }`. Fill the parameter list and body at the tabstops.

```sh
codemmet expand --lang javascript "arrow:double>ret"
```

```javascript
const double = () => {
  return ;
};
```

Composing `>ret` drops a `return` statement into the arrow body.

### `aw`

An await statement.

```sh
codemmet expand --lang javascript "aw"
```

```javascript
await ;
```

`aw` expands to a bare `await` expression. Fill the tabstop with the promise you want to resolve.

### `case`

A case clause with break.

```sh
codemmet expand --lang javascript "case"
```

```javascript
case value:
  
  break;
```

`case` expands to a single `case` label with a body tabstop and a trailing `break`. On its own it renders the clause without an enclosing switch.

```sh
codemmet expand --lang javascript "switch:status>case+default"
```

```javascript
switch (status) {
  case value:
    
    break;
  default:
    
}
```

Placing `case` (and a sibling `default`) inside a `switch` builds the full statement, with each branch indented into the switch body.

### `class`

A class definition.

```sh
codemmet expand --lang javascript "class:Store"
```

```javascript
class Store {
  
}
```

The glued argument names the class, so `class:Store` produces `class Store { … }` with a body tabstop for its members.

```sh
codemmet expand --lang javascript "class:Store>ctor+fn:add"
```

```javascript
class Store {
  constructor() {
    
  }
  add() {
    
  }
}
```

Children of a class become members: `ctor` is the constructor and `fn:add` becomes a method (no `function` keyword) because it is composed inside the class.

### `ctor`

A constructor method.

```sh
codemmet expand --lang javascript "ctor"
```

```javascript
constructor() {
  
}
```

`ctor` expands to a `constructor()` method with a body tabstop. On its own it renders just the method.

```sh
codemmet expand --lang javascript "class:Point>ctor"
```

```javascript
class Point {
  constructor() {
    
  }
}
```

Composed inside `class:Point`, the constructor is placed as a member of the class body.

### `default`

A default clause.

```sh
codemmet expand --lang javascript "default"
```

```javascript
default:
  
```

`default` expands to a `default:` label with a body tabstop, the fallthrough branch of a switch.

```sh
codemmet expand --lang javascript "switch:kind>default"
```

```javascript
switch (kind) {
  default:
    
}
```

Composed inside a `switch`, the `default` branch is indented into the switch body.

### `doc`

A JSDoc comment line.

```sh
codemmet expand --lang javascript "doc"
```

```javascript
/**  */
```

`doc` expands to a one-line JSDoc block comment. Fill the tabstop between the delimiters with the description.

### `el`

An else block.

```sh
codemmet expand --lang javascript "el"
```

```javascript
else {
  
}
```

`el` expands to a standalone `else { … }` block with a body tabstop.

```sh
codemmet expand --lang javascript "if+el"
```

```javascript
if (condition) {
  
}
else {
  
}
```

Using the `+` sibling operator places the `else` right after an `if`, forming a complete if/else.

### `elif`

An else-if block.

```sh
codemmet expand --lang javascript "elif"
```

```javascript
else if (condition) {
  
}
```

`elif` expands to an `else if (condition) { … }` block with tabstops for the condition and body.

```sh
codemmet expand --lang javascript "if+elif"
```

```javascript
if (condition) {
  
}
else if (condition) {
  
}
```

As a sibling of `if`, the `elif` chains a second condition after the initial branch.

### `exp`

Export a const binding.

```sh
codemmet expand --lang javascript "exp:API_URL"
```

```javascript
export const API_URL = ;
```

The glued argument names the binding, so `exp:API_URL` produces `export const API_URL = …`. Fill the tabstop with the value.

### `expd`

Export default.

```sh
codemmet expand --lang javascript "expd"
```

```javascript
export default ;
```

`expd` expands to an `export default` statement. Fill the tabstop with the value or expression to export.

### `fn`

A function declaration.

```sh
codemmet expand --lang javascript "fn:render"
```

```javascript
function render() {
  
}
```

The glued argument names the function, so `fn:render` produces `function render() { … }` with tabstops for the parameters and body.

```sh
codemmet expand --lang javascript "fn:render>ret"
```

```javascript
function render() {
  return ;
}
```

Composing `>ret` drops a `return` statement into the function body.

### `fore`

A for...of loop.

```sh
codemmet expand --lang javascript "fore"
```

```javascript
for (const item of items) {
  
}
```

`fore` expands to a `for...of` loop with default `item` and `items` names. A glued argument renames the loop variable.

```sh
codemmet expand --lang javascript "fore:row>pr"
```

```javascript
for (const row of items) {
  console.log();
}
```

Here the argument renames the element to `row`, and the composed `pr` drops a `console.log()` into the loop body.

### `fori`

A for loop over an integer range.

```sh
codemmet expand --lang javascript "fori"
```

```javascript
for (let i = 0; i < 10; i++) {
  
}
```

`fori` expands to a counting `for` loop. A glued number sets the upper bound of the range.

```sh
codemmet expand --lang javascript "fori10>pr"
```

```javascript
for (let i = 0; i < 10; i++) {
  console.log();
}
```

`fori10` fixes the bound at `10`, and the composed `pr` logs inside each iteration.

### `forin`

A for...in loop over object keys.

```sh
codemmet expand --lang javascript "forin"
```

```javascript
for (const key in object) {
  
}
```

`forin` expands to a `for...in` loop with default `key` and `object` names. A glued argument renames the key variable.

```sh
codemmet expand --lang javascript "forin:prop>pr"
```

```javascript
for (const prop in object) {
  console.log();
}
```

Here the argument renames the key to `prop`, and the composed `pr` logs inside the loop.

### `get`

A getter backed by a private-by-convention field.

```sh
codemmet expand --lang javascript "get"
```

```javascript
get value() {
  return this._value;
}
```

`get` expands to a `value` getter that returns `this._value`, the underscore-prefixed backing field.

```sh
codemmet expand --lang javascript "class:Box>get"
```

```javascript
class Box {
  get value() {
    return this._value;
  }
}
```

Composed inside `class:Box`, the getter is placed as a member of the class body.

### `if`

An if block.

```sh
codemmet expand --lang javascript "if"
```

```javascript
if (condition) {
  
}
```

`if` expands to an `if (condition) { … }` block with tabstops for the condition and body.

```sh
codemmet expand --lang javascript "if>ret"
```

```javascript
if (condition) {
  return ;
}
```

Composing `>ret` places a `return` inside the guarded block, the common early-return shape.

### `imp`

A default import.

```sh
codemmet expand --lang javascript "imp:react"
```

```javascript
import name from "react";
```

The glued argument fills the module specifier, so `imp:react` imports from `"react"`. Fill the `name` tabstop with the local binding.

### `impd`

A destructured (named) import.

```sh
codemmet expand --lang javascript "impd:react"
```

```javascript
import {  } from "react";
```

The glued argument sets the module, so `impd:react` produces `import { … } from "react"`. Fill the braces with the named bindings.

### `jsonp`

A JSON.parse expression.

```sh
codemmet expand --lang javascript "jsonp"
```

```javascript
JSON.parse()
```

`jsonp` expands to a `JSON.parse()` call. Fill the tabstop with the string to parse.

### `jsons`

A JSON.stringify expression.

```sh
codemmet expand --lang javascript "jsons"
```

```javascript
JSON.stringify()
```

`jsons` expands to a `JSON.stringify()` call. Fill the tabstop with the value to serialize.

### `pr`

A console.log call.

```sh
codemmet expand --lang javascript "pr"
```

```javascript
console.log();
```

`pr` expands to a `console.log()` statement. Fill the tabstop with the arguments to log.

### `prd`

A debug-log with an object-shorthand label.

```sh
codemmet expand --lang javascript "prd"
```

```javascript
console.log({  });
```

`prd` logs an object literal, so `{ value }` object shorthand prints both the name and the value of what you drop in.

### `prom`

A new Promise with an executor.

```sh
codemmet expand --lang javascript "prom"
```

```javascript
new Promise((resolve, reject) => {
  
})
```

`prom` expands to a `new Promise` whose executor takes `resolve` and `reject`. The body tabstop is where you settle the promise.

```sh
codemmet expand --lang javascript "fn:delay>prom"
```

```javascript
function delay() {
  new Promise((resolve, reject) => {
    
  })
}
```

Composed inside `fn:delay`, the promise is nested in the function body, ready to be returned once you add the work.

### `ret`

A return statement.

```sh
codemmet expand --lang javascript "ret"
```

```javascript
return ;
```

`ret` expands to a bare `return` statement. Fill the tabstop with the value to return.

### `set`

A setter backed by a private-by-convention field.

```sh
codemmet expand --lang javascript "set"
```

```javascript
set value(value) {
  this._value = value;
}
```

`set` expands to a `value` setter that assigns its argument to `this._value`, the underscore-prefixed backing field.

```sh
codemmet expand --lang javascript "class:Box>set"
```

```javascript
class Box {
  set value(value) {
    this._value = value;
  }
}
```

Composed inside `class:Box`, the setter is placed as a member of the class body.

### `switch`

A switch statement.

```sh
codemmet expand --lang javascript "switch:method"
```

```javascript
switch (method) {
  
}
```

The glued argument sets the discriminant, so `switch:method` produces `switch (method) { … }` with a body tabstop for the branches.

```sh
codemmet expand --lang javascript "switch:method>case+default"
```

```javascript
switch (method) {
  case value:
    
    break;
  default:
    
}
```

Child `case` and `default` triggers fill the switch body, each becoming its own branch.

### `test`

A test case (jest/vitest style).

```sh
codemmet expand --lang javascript "test"
```

```javascript
test("works", () => {
  
});
```

`test` expands to a `test(name, callback)` call with a default `"works"` description and an arrow-function body.

```sh
codemmet expand --lang javascript "test>aeq"
```

```javascript
test("works", () => {
  expect(actual).toBe();
});
```

Composing `>aeq` drops an equality expectation inside the test body.

### `todo`

A TODO comment.

```sh
codemmet expand --lang javascript "todo"
```

```javascript
// TODO: 
```

`todo` expands to a `// TODO:` line comment. Fill the tabstop with the note.

### `try`

A try/catch block.

```sh
codemmet expand --lang javascript "try"
```

```javascript
try {
  
} catch (err) {
  
}
```

`try` expands to a `try { … } catch (err) { … }` block with body tabstops in both the try and the catch.

```sh
codemmet expand --lang javascript "afn:load>try"
```

```javascript
async function load() {
  try {
    
  } catch (err) {
    
  }
}
```

Composed inside `afn:load`, the try/catch wraps the async function body, the typical place to await a call that may reject.

### `while`

A while loop.

```sh
codemmet expand --lang javascript "while"
```

```javascript
while (condition) {
  
}
```

`while` expands to a `while (condition) { … }` loop with tabstops for the condition and body.

```sh
codemmet expand --lang javascript "while>pr"
```

```javascript
while (condition) {
  console.log();
}
```

Composing `>pr` drops a `console.log()` into the loop body.

## See also

- [abbreviation syntax](../syntax.md)
- [language reference](../languages.md)
- [examples overview](README.md)
