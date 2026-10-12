# Richer world history

- **Request**: give generated history more templates per event kind, named events with backstories and causal chains (1.0 task 16, OI-35).
- **Templates**: every event kind has at least 4 text templates and every titled kind at least 3; lore families have at least 4; 12 war-name patterns, 18 plague names, 7 ruin kinds per tier. A linter in `Tables::check` refuses unknown slots, empty families and unused keys.
- **Events**: new optional `name`, `backstory` and `cause_kind` fields on `Event` (additive, `format_version` unchanged). Plagues, wars, famines, severe floods and fires and dynasty changes are named uniquely; plagues, famines, floods, fires, mine collapses and wars say where they came from.
- **Causes**: planned famines and plagues move to just after a flood, war or famine of their realm (flood or war -> famine -> plague -> abandonment); borders and present realms are unchanged. Settlement stories, region hooks and ruin lore use the names and backstories.
- **Bytes**: output differs from before in event text, titles and the new fields, and in the years of famines and plagues that were moved into a chain.
