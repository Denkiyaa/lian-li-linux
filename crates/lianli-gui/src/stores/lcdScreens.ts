import { defineStore } from "pinia";
import { ref } from "vue";

const NAMES_KEY = "lianli.lcdNames";
const ORDER_KEY = "lianli.lcdOrder";
const MAX_NAME_LENGTH = 40;

function readJson(key: string): unknown {
  try {
    return JSON.parse(localStorage.getItem(key) ?? "null");
  } catch {
    return null;
  }
}

function writeJson(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Screen names and order are display preferences; they stay in memory when storage is unavailable.
  }
}

function readNames(): Record<string, string> {
  const stored = readJson(NAMES_KEY);
  if (!stored || typeof stored !== "object" || Array.isArray(stored)) return {};
  return Object.fromEntries(
    Object.entries(stored).filter((entry): entry is [string, string] => typeof entry[1] === "string"),
  );
}

function readOrder(): string[] {
  const stored = readJson(ORDER_KEY);
  return Array.isArray(stored) ? stored.filter((key): key is string => typeof key === "string") : [];
}

export const useLcdScreensStore = defineStore("lcdScreens", () => {
  const names = ref<Record<string, string>>(readNames());
  const order = ref<string[]>(readOrder());

  function setName(key: string, name: string) {
    const trimmed = name.trim().slice(0, MAX_NAME_LENGTH);
    const next = { ...names.value };
    if (trimmed) next[key] = trimmed;
    else delete next[key];
    names.value = next;
    writeJson(NAMES_KEY, next);
  }

  function setOrder(keys: string[]) {
    order.value = [...keys];
    writeJson(ORDER_KEY, order.value);
  }

  function carryOver(from: string, to: string) {
    if (from === to) return;
    const name = names.value[from];
    if (name && !names.value[to]) setName(to, name);
    const position = order.value.indexOf(from);
    if (position !== -1 && !order.value.includes(to)) {
      setOrder([...order.value.slice(0, position + 1), to, ...order.value.slice(position + 1)]);
    }
  }

  return { names, order, setName, setOrder, carryOver };
});
