# Property templates

`property-template:v1` files are bounded authoring helpers. They expand into
`author-property:v1` declarations and are re-parsed by the existing schema;
they do not create a property claim or node evidence.

```bash
ergo-es property-template examples/property-templates/reserve-conservation.json --json
```

A placeholder occupies a complete JSON string leaf, such as `{{property_id}}`.
The base fixes the declaration's fields and nesting; the expanded result is
still subject to duplicate-key rejection, normalization, expression typing and
all existing property ceilings.
