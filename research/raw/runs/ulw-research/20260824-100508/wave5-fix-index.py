import io

def patch(path, pairs):
    c = io.open(path, encoding='utf-8').read()
    for old, new in pairs:
        assert old in c, f"MISSING {path}: {old[:70]}"
        c = c.replace(old, new, 1)
    io.open(path, 'w', encoding='utf-8', newline='').write(c)

patch('research/surveys/00-index.md', [
 ('| [01-coding-agents.md](01-coding-agents.md) | Coding agents & harnesses (terminal/IDE/cloud/companions) | 31 |',
  '| [01-coding-agents.md](01-coding-agents.md) | Coding agents & harnesses (terminal/IDE/cloud/companions) | 33 |'),
 ('| [02-memory-context.md](02-memory-context.md) | Memory systems & context/token engineering | 10 |',
  '| [02-memory-context.md](02-memory-context.md) | Memory systems & context/token engineering | 12 |'),
 ('| [04-local-inference.md](04-local-inference.md) | Local inference runtimes & model tooling | 10 |',
  '| [04-local-inference.md](04-local-inference.md) | Local inference runtimes & model tooling | 11 |'),
 ('| [06-orchestration.md](06-orchestration.md) | Orchestration frameworks, workflow engines, observability | 19 |',
  '| [06-orchestration.md](06-orchestration.md) | Orchestration frameworks, workflow engines, observability | 22 |'),
 ('| [07-media-apps.md](07-media-apps.md) | Media gen/editing, office suites, end-user AI apps | 18 |',
  '| [07-media-apps.md](07-media-apps.md) | Media gen/editing, office suites, end-user AI apps | 20 |'),
 ('**130 active entries + 2 merges + 2 delists**',
  '**142 active entries + 2 merges + 2 delists**'),
])
print('index synced')
