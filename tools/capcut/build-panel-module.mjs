import fs from 'node:fs';
import path from 'node:path';

const panels = JSON.parse(fs.readFileSync('catalog/panels.json', 'utf8'));
const catalog = JSON.parse(fs.readFileSync('catalog/capcut-catalog.json', 'utf8'));
const COVERS = 'C:/Users/aitor/Documents/deepseek-harness/default-workspace/capcut-dl/covers-panels';
const catName = new Map(catalog.categories.map((c) => [String(c.id), c.name]));

const build = (key) => {
  const panel = panels.panels[key];
  if (!panel) return { cats: [], items: [] };
  const dir = path.join(COVERS, key);
  const items = panel.items
    .filter((item) => fs.existsSync(path.join(dir, `${item.id}.webp`)))
    .map((item) => ({
      id: item.id, name: item.name, cat: item.cat, cats: item.cats,
      img: `assets/capcut/panels/${key}/${item.id}.webp`, url: item.cover,
    }));
  const cats = panel.categories.map((c) => c.name).filter((name) => items.some((i) => i.cats.includes(name)));
  return { cats, items };
};

const transitions = build('transitions');
const filters = build('filters');
const stickers = build('stickers');
const text = build('text');

// Efectos: web (1.834) + escritorio (662), cada uno con su lista de categorías.
const web = JSON.parse(fs.readFileSync('catalog/capcut-web-effects.json', 'utf8'));
const webCat = new Map((web.categories ?? []).map((c) => [String(c.id), c.name]));
const webCovers = 'C:/Users/aitor/Documents/deepseek-harness/default-workspace/capcut-dl/covers-web';
const webItems = web.effects
  .filter((e) => fs.existsSync(path.join(webCovers, `${e.id}.jpg`)))
  .map((e) => {
    const names = [...new Set((e.categories ?? []).map((id) => webCat.get(String(id)) ?? catName.get(String(id))).filter(Boolean))];
    return { id: e.id, name: e.title || '(sin nombre)', cat: names[0] ?? 'Efectos', cats: names.length ? names : ['Efectos'], img: `assets/capcut/covers/${e.id}.jpg`, url: e.cover };
  });

const desktop = build('effects');
const seen = new Set(webItems.map((i) => i.id));
const extraItems = desktop.items.filter((i) => !seen.has(i.id));
const effectItems = [...webItems, ...extraItems];
const effectCats = [...new Set([...desktop.cats, ...webItems.flatMap((i) => i.cats)])]
  .filter((name) => effectItems.some((i) => i.cats.includes(name)));

const body = `// Catálogo de CapCut: paneles completos, portadas reales y categorías en el orden de CapCut.
// Efectos ${effectItems.length} · Transiciones ${transitions.items.length} · Filtros ${filters.items.length} · Stickers ${stickers.items.length} · Texto ${text.items.length}

export const CAPCUT_EFFECT_CATEGORIES = ${JSON.stringify(['Todos', ...effectCats])};
export const CAPCUT_EFFECT_ITEMS = ${JSON.stringify(effectItems)};

export const CAPCUT_TRANSITION_CATEGORIES = ${JSON.stringify(['Todas', ...transitions.cats])};
export const CAPCUT_TRANSITION_ITEMS = ${JSON.stringify(transitions.items)};

export const CAPCUT_FILTER_CATEGORIES = ${JSON.stringify(['Todos', ...filters.cats])};
export const CAPCUT_FILTER_ITEMS = ${JSON.stringify(filters.items)};

export const CAPCUT_STICKER_CATEGORIES = ${JSON.stringify(['Todos', ...stickers.cats])};
export const CAPCUT_STICKER_ITEMS = ${JSON.stringify(stickers.items)};

export const CAPCUT_TEXT_CATEGORIES = ${JSON.stringify(['Todos', ...text.cats])};
export const CAPCUT_TEXT_ITEMS = ${JSON.stringify(text.items)};
`;

fs.writeFileSync('capcut-panels.js', body);
console.log('efectos', effectItems.length, 'cats', effectCats.length);
console.log('transiciones', transitions.items.length, '| filtros', filters.items.length, '| stickers', stickers.items.length, '| texto', text.items.length);
