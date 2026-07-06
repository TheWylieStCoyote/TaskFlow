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

## See also

- [abbreviation syntax](../syntax.md)
- [language reference](../languages.md)
- [examples overview](README.md)
