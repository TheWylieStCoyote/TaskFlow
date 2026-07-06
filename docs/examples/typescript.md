# TypeScript examples

The `typescript` language handles files with the `ts`, `mts`, `cts`, and `tsx` extensions, and matches the LSP ids `typescript` and `typescriptreact`. Pass `--lang typescript`, or let codemmet match on any of those ids or extensions. Its default indent is 2 spaces, and it inherits the JavaScript pack, so every [JavaScript](javascript.md) trigger is available here too. See the [language reference](../languages.md) for the full list of triggers.

> Examples use the default `strip` mode, which renders plain code and leaves an empty spot wherever a tabstop will be — those are the places you fill in. Add `--tabstops keep` to see them as `$1` / `${1:placeholder}` / `$0` markers. See the [examples overview](README.md) for more.

## An interface

```sh
codemmet expand --lang typescript "int:User"
```

```typescript
interface User {
  
}
```

The `int` trigger takes `User` as a glued argument for the name and opens an interface body. The tabstop inside is where you list the members.

## A class with a typed async method

```sh
codemmet expand --lang typescript "class:Service>ctor+afn:fetch>try"
```

```typescript
class Service {
  constructor() {
    
  }
  async fetch(): Promise<void> {
    try {
      
    } catch (err) {
      
    }
  }
}
```

The `afn` here produces a typed `: Promise<void>` signature: TypeScript's pack overrides the JavaScript `afn` it inherits. The method sits beside the `ctor` as a sibling, with `try` nesting a try/catch in its body.

## A typed class method

```sh
codemmet expand --lang typescript "class:Repo>fn:find>ret"
```

```typescript
class Repo {
  find(): void {
    return ;
  }
}
```

Composed inside a class, `fn` picks the method form and gains a `: void` return annotation from the TypeScript pack. The nested `ret` adds a `return` statement with a tabstop for the value.

## A type alias and a generic function

```sh
codemmet expand --lang typescript "typ+fng:identity>ret"
```

```typescript
type Name = ;
function identity<T>(): T {
  return ;
}
```

The `typ` trigger starts a type alias, and `fng` expands to a generic function with a `<T>` parameter and a `: T` return type. The `identity` glued argument names the function; fill the alias value and the returned expression at the tabstops.

## An enum

```sh
codemmet expand --lang typescript "enum:Direction"
```

```typescript
enum Direction {
  
}
```

The `enum` trigger takes `Direction` as a glued argument and opens an enum body. Add your members at the tabstop inside.

## See also

- [abbreviation syntax](../syntax.md)
- [language reference](../languages.md)
- [examples overview](README.md)
