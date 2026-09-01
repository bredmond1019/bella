# Bella Reference Demo

A single document exercising every render feature bella supports, used to
generate the reference screenshot set in `planning/artifacts/screenshots/`.

## Headings

### H3 heading
#### H4 heading
##### H5 heading
###### H6 heading

## Text styles

Plain text, **bold text**, *italic text*, ***bold italic***, ~~strikethrough~~,
and `inline code`.

> A blockquote, to show the quote bar and quote color.
>
> > A nested blockquote one level deeper.

## Links

Visit [the bella repo](https://github.com/bredmond1019/bella) or read
[the README](README.md) locally.

## Lists

- Top-level bullet
  - Nested bullet
    - Doubly-nested bullet
- Another top-level bullet

1. First ordered item
2. Second ordered item
   1. Nested ordered item

## Task list

- [ ] An unchecked task
- [x] A checked task
- [ ] Another unchecked task

## Code block

```rust
fn main() {
    let theme = Theme::dark();
    println!("cool-aurora, {:?}", theme.name);
}
```

## Table

| Requirement | Why | If missing |
|---|---|---|
| Rust stable toolchain | Building the workspace | Install via rustup |
| A terminal with mouse support | Hover/click/drag gestures | Most modern emulators work |
| A system clipboard provider | Drag-select / double-click copy | On Linux without a display server, install one |

## Horizontal rule

---

End of demo document.
