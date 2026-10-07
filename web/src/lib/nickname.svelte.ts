const STORAGE_KEY = "vynl.nickname";

const ADJECTIVES = [
  "Cosmic",
  "Velvet",
  "Neon",
  "Quiet",
  "Golden",
  "Midnight",
  "Paper",
  "Wild",
  "Silver",
  "Lunar",
  "Electric",
  "Soft",
  "Amber",
  "Static",
  "Marble",
  "Feral",
  "Polar",
  "Analog",
  "Distant",
  "Tidal",
  "Crimson",
  "Hollow",
  "Mellow",
  "Restless",
  "Sable",
  "Bright",
  "Dusty",
  "Endless",
  "Faint",
  "Frozen",
  "Gentle",
  "Heavy",
  "Hidden",
  "Iron",
  "Lonesome",
  "Muddy",
  "Nimble",
  "Opal",
  "Patient",
  "Plain",
  "Quietly",
  "Rough",
  "Sleepy",
  "Steady",
  "Stony",
  "Sunken",
  "Teal",
  "Thin",
  "Wandering",
  "Woven",
  "Arctic",
  "Blurred",
  "Copper",
  "Drowsy",
  "Eager",
  "Faintly",
  "Gilded",
  "Hazy",
  "Idle",
  "Jaded",
  "Lucid",
  "Mossy",
  "Narrow",
  "Pearly",
  "Quietest",
  "Ragged",
  "Sandy",
  "Silent",
  "Tender",
  "Vacant",
  "Wintry",
] as const;

const NOUNS = [
  "Panda",
  "Echo",
  "Harbor",
  "Comet",
  "Meadow",
  "Signal",
  "Lantern",
  "Orchid",
  "Cinder",
  "Drift",
  "Willow",
  "Ember",
  "Atlas",
  "Fathom",
  "Prism",
  "Nomad",
  "Canyon",
  "Verge",
  "Halo",
  "Badger",
  "Beacon",
  "Cedar",
  "Cove",
  "Dahlia",
  "Delta",
  "Dune",
  "Estuary",
  "Fern",
  "Fjord",
  "Foundry",
  "Grove",
  "Harbour",
  "Hollow",
  "Isle",
  "Jetty",
  "Kite",
  "Lagoon",
  "Lark",
  "Lichen",
  "Mantis",
  "Meridian",
  "Monsoon",
  "Moss",
  "Nettle",
  "Onyx",
  "Owl",
  "Pebble",
  "Petrel",
  "Quarry",
  "Reef",
  "Ridge",
  "Rill",
  "Sparrow",
  "Summit",
  "Thicket",
  "Thistle",
  "Tundra",
  "Vesper",
  "Vista",
  "Wavelength",
  "Whistler",
  "Willowby",
  "Yarrow",
  "Zephyr",
  "Basil",
  "Bramble",
  "Clover",
  "Fennel",
  "Hare",
  "Heron",
  "Ibis",
  "Juniper",
  "Kestrel",
  "Moth",
  "Osprey",
  "Otter",
  "Raven",
  "Rowan",
  "Sorrel",
  "Vireo",
  "Wombat",
  "Wren",
] as const;

function pick<T>(items: readonly T[]): T {
  return items[Math.floor(Math.random() * items.length)];
}

function randomNickname(): string {
  return `${pick(ADJECTIVES)} ${pick(NOUNS)}`;
}

function load(): string {
  if (typeof localStorage === "undefined") return randomNickname();
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored && stored.trim()) return stored;
  const generated = randomNickname();
  localStorage.setItem(STORAGE_KEY, generated);
  return generated;
}

let _nickname = $state<string | null>(null);

export function getNickname(): string {
  if (_nickname === null) _nickname = load();
  return _nickname;
}

export function rollNickname(): string {
  _nickname = randomNickname();
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(STORAGE_KEY, _nickname);
  }
  return _nickname;
}

export function resolveDisplayName(configured: string): string {
  const name = configured?.trim();
  return name ? name : getNickname();
}