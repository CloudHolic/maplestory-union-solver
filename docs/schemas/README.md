# Schemas

Canonical data format definitions shared between the three languages in
this project (Rust, Python, TypeScript), expressed as JSON Schema.

## Why JSON Schema

- Language-neutral: all three languages have mature code generators
  (`schemars` + `typify` for Rust, `datamodel-code-generator` for Python,
  `json-schema-to-typescript` for TypeScript).
- Runtime validation: ajv, jsonschema, jsonschema-rs.
- Meaningful version diffs when schemas change.

## Versioning

Breaking changes are versioned by suffix:

```
solve-log.json         (current, symlink to latest)
solve-log-v1.json      (frozen)
solve-log-v2.json      (frozen)
```

JSONL log lines include a `"schema_version": "v1"` field to identify
producing schema.

## Change coordination

Modifying a schema requires updating code in all three languages. A
checklist is enforced in code review:

- [ ] `wasm/` consumer/producer updated
- [ ] `ml/` consumer/producer updated
- [ ] `ui/` consumer/producer updated
- [ ] Version suffix incremented if breaking
