import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { setTimeout as delay } from "node:timers/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/utils/brightnessControl.ts", import.meta.url), "utf8");
const javascript = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
const { createBrightnessControl, brightnessError, brightnessConfirmed } = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString("base64")}`);
assert.equal(brightnessError(30, undefined, "LCD not found"), "LCD not found");
assert.equal(brightnessError(30, { brightness: 30, pending: true, error: "USB write failed" }), "USB write failed");
assert.equal(brightnessError(30, { brightness: 30, pending: false, error: "Retries exhausted" }), "Retries exhausted");
assert.equal(brightnessError(30, { brightness: 30, pending: false, error: null }, "USB write failed"), "USB write failed");
const oldConfirmation = { request_id: "old-request", brightness: 30, pending: false, error: null };
assert.equal(brightnessError(30, oldConfirmation, "Daemon stopped", "new-request"), "Daemon stopped");
const newConfirmation = { ...oldConfirmation, request_id: "new-request" };
assert.equal(brightnessConfirmed(oldConfirmation, "new-request"), false);
assert.equal(brightnessConfirmed(newConfirmation, "new-request"), true);
assert.equal(brightnessConfirmed({ ...newConfirmation, pending: true }, "new-request"), false);
assert.equal(brightnessConfirmed({ ...newConfirmation, error: "USB write failed" }, "new-request"), false);
assert.equal(brightnessConfirmed({ brightness: 30, pending: false, error: null }), false);
assert.equal(brightnessError(30, newConfirmation, "USB write failed", "new-request"), undefined);
assert.equal(brightnessError(30, { ...newConfirmation, pending: true }, "USB write failed", "new-request"), "USB write failed");
assert.equal(brightnessError(30, { ...newConfirmation, error: "Retries exhausted" }, "USB write failed", "new-request"), "Retries exhausted");
assert.equal(brightnessError(80, newConfirmation, "New request failed", "new-request"), "New request failed");
assert.equal(brightnessError(80, { brightness: 30, pending: false, error: "Old failure" }), undefined);
const sent = [];
const errors = [];
const releases = [];
const control = createBrightnessControl(
  (device, value) => {
    sent.push([device, value]);
    return new Promise((resolve, reject) => releases.push({ resolve, reject }));
  },
  (device, error) => errors.push([device, error]),
);

for (let value = 0; value <= 100; value++) control.set("head", value);
await delay(250);
assert.deepEqual(sent, [["head", 100]]);
for (let value = 100; value >= 0; value--) control.set("head", value);
control.set("other", 37);
await delay(250);
assert.equal(sent.length, 1, "no overlapping requests while delivery is blocked");
releases.shift().reject("old request expired");
await delay(250);
assert.deepEqual(errors, [], "superseded failures do not obscure the new request");
assert.deepEqual(sent.at(-1), ["head", 0]);
releases.shift().resolve();
await delay(250);
assert.deepEqual(sent.at(-1), ["other", 37]);
releases.shift().reject("LCD not found");
await delay(0);
assert.deepEqual(errors, [["other", "LCD not found"]]);
control.set("other", 80);
await delay(250);
assert.deepEqual(sent.at(-1), ["other", 80], "a later request still runs after an error");
control.set("head", 50);
control.dispose();
releases.shift().reject("shutdown");
await delay(250);
assert.equal(sent.length, 4, "disposal drops pending changes");
assert.equal(errors.length, 1, "disposal suppresses late errors");
console.log("Brightness coalescing, serialization, failures and disposal passed");
