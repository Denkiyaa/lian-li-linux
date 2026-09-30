import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/stores/sensorOptions.ts", import.meta.url), "utf8");
const javascript = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 },
}).outputText;
const { decodeOption, enumerateSensorsAsOptions, optionForConfig, sourceConfigsAsOptions } =
  await import(`data:text/javascript;base64,${Buffer.from(javascript).toString("base64")}`);

for (const [source, config] of [
  [{ type: "network_rate", iface: "enp14s0", direction: "rx" }, { type: "network_rx", iface: "enp14s0" }],
  [{ type: "network_rate", iface: "enp14s0", direction: "tx" }, { type: "network_tx", iface: "enp14s0" }],
  [{ type: "disk_rate", device: "nvme0n1", direction: "read" }, { type: "disk_read", device: "nvme0n1" }],
  [{ type: "disk_rate", device: "nvme0n1", direction: "write" }, { type: "disk_write", device: "nvme0n1" }],
  [{ type: "hwmon", name: "k10temp", label: "Tctl", device_path: "pci-0000:00:18.3" },
   { type: "hwmon", name: "k10temp", label: "Tctl", device_path: "pci-0000:00:18.3" }],
]) {
  const sensors = [{ source, unit: "MBps", display_name: "fixture" }];
  const option = enumerateSensorsAsOptions(sensors, false)[0];
  assert.deepEqual(decodeOption(option.value), config);
  assert.equal(optionForConfig(sensors, config), option.value);
  assert.deepEqual(JSON.parse(sourceConfigsAsOptions(sensors)[0].value), config);
  assert.deepEqual(decodeOption(JSON.stringify(config)), config);
}
assert.equal(decodeOption("command"), null);
assert.equal(decodeOption("invalid JSON"), null);
assert.equal(optionForConfig([], { type: "cpu_usage" }), "");
assert.deepEqual(sourceConfigsAsOptions([], true), [{ label: "Custom command", value: "command" }]);
console.log("Sensor picker configuration conversion and saved selections passed");
