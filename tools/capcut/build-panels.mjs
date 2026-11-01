import fs from 'node:fs';

const cat = JSON.parse(fs.readFileSync('catalog/capcut-catalog.json', 'utf8'));
const catName = new Map(cat.categories.map((c) => [String(c.id), c.name]));
const byId = new Map(cat.effects.map((e) => [e.id, e]));
const panelById = new Map(cat.panels.map((p) => [p.panelId, p]));

const GROUPS = {
  effects: ['1635', '3892', '3884'],
  transitions: ['4486'],
  filters: ['1923'],
  stickers: ['4721', '4442'],
  text: ['4562'],
};

const out = { generatedAt: new Date().toISOString(), panels: {} };

for (const [key, ids] of Object.entries(GROUPS)) {
  const panels = ids.map((id) => panelById.get(id)).filter(Boolean);
  if (!panels.length) continue;
  const categories = [];
  const seenCats = new Set();
  for (const panel of panels) {
    for (const c of panel.categories) {
      if (seenCats.has(c.name)) continue;
      seenCats.add(c.name);
      categories.push({ id: String(c.id), name: c.name });
    }
  }
  const orderIndex = new Map(categories.map((c, i) => [c.name, i]));
  const items = new Map();
  for (const c of categories) {
    for (const e of cat.effects) {
      if (!e.categories.includes(c.id) || !e.coverUrl) continue;
      const names = [...new Set(e.categories.map((id) => catName.get(id)).filter(Boolean))];
      if (!items.has(e.id)) {
        items.set(e.id, {
          id: e.id, name: e.title || '(sin nombre)', cover: e.coverUrl, type: e.type,
          cats: names.filter((n) => orderIndex.has(n)),
        });
      }
    }
  }
  const list = [...items.values()].filter((item) => item.cats.length);
  for (const item of list) item.cat = item.cats[0];
  out.panels[key] = { categories: categories.filter((c) => list.some((i) => i.cats.includes(c.name))), items: list };
  const empty = out.panels[key].categories.filter((c) => !list.some((i) => i.cats.includes(c.name)));
  console.log(`${key}: ${list.length} items, ${out.panels[key].categories.length} cats (vacías: ${empty.length})`);
}

fs.writeFileSync('catalog/panels.json', JSON.stringify(out, null, 1));
