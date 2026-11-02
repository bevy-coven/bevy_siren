# Decisions the loop will not make

One markdown file per question the loop hit and could not settle itself, named `<short-slug>.md`. The loop writes it,
keeps building the rest of the item, and says in `log.md` which item waits on it.

Each file answers four questions, in this order:

1. **The question**, in one sentence a person can answer yes or no to, or choose between.
2. **The options**, each with what it costs: a consumer's callsite, a migration of a project's `family/bevy_siren.ron`,
   a performance or a behaviour tradeoff.
3. **The recommendation**, and the source or the precedent that argues for it.
4. **What is blocked**, naming the code that cannot be written until the answer arrives, and what the loop did
   instead.

A decision file is never a way to stop working. The loop decides anything reversible and internal by itself, and says
so in `log.md`; only the public API, the message types and the definitions-file schema come here.
