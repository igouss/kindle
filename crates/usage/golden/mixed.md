## What

The watcher runs `br list --status in_progress` per workspace:

```
br list --status in_progress --json
  --no-auto-flush --no-auto-import
```

> A read-only list raises only CLOSE_WRITE; a real update raises MODIFY.

1. Wake on MODIFY or MOVED_TO.
   - never on CLOSE_WRITE
   - debounce 300 ms
2. Push only on change.

| Event | Wakes |
| --- | --- |
| MODIFY | yes |
| CLOSE_WRITE | no |

---

See https://github.com/igouss/franken_markdown/blob/4b705d147c10cebb52866f37dbdb1492ce8505ea/src/ast.rs for the AST.
