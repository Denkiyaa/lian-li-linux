# OpenRGB lighting control

Enable the OpenRGB SDK server in RGB or Settings and connect OpenRGB to its configured port (6743 by default). Lian Li Linux continues handling cooling and the LCD. Disable OpenRGB's native detectors for devices already controlled by this daemon.

## AL V2 inner and outer regions

Enable **AL V2 inner/outer regions** and save. Each populated AL V2 fan group appears as one device with Inner and Outer zones in OpenRGB when connected using SDK v6. Select a zone to change its effect independently.

Devices whose drivers support only zone colours expose one logical colour per nonempty zone, rather than independently addressable LEDs. For example, the Galahad II Vision AIO exposes Pump Head and Fans colours. Its physical fan LED count remains configurable in Lian Li Linux and still controls hardware animations. True per-LED devices keep their full layout. Recreate OpenRGB profiles for devices whose logical LED count changes.

Static and Breathing support one colour per fan in each region. Other animations run across the region and offer the palettes, speed and direction controls supported by the driver.

Palette effects start with editable colours and remember their palette when you switch modes during the session. OpenRGB labels palette slots **Mode-Specific Color 0**, **1**, and so on; these are animation colours, not fan numbers. Use the colour-count control to change the palette size, then select each slot to edit it. Explicitly empty palettes and black colours remain valid, including when loaded from a profile. Off has no palette controls. AL V2 Rainbow, Rainbow Morph and Meteor Rainbow generate their own colours and also have no palette controls. Re-save older profiles using these modes after checking brightness and speed, because OpenRGB does not restore parameters whose mode metadata has changed.

**Entire Device** offers effects supported by both regions. Selecting an effect there applies it to both. Selecting a zone effect overrides that region; **Follow Device Mode** restores the device's current effect for that zone. Effects supported by only one region remain available on that zone.

Older SDK clients (v0–5) see separate Inner and Outer devices. Both layouts share the same lighting state.

Devices whose drivers advertise independent zone effects also expose those modes in SDK v6. On a supported AIO, select **Pump Head** or **Fans** to set that zone's effect, brightness and speed. **Entire Device** applies a mode to both zones; **Follow Device Mode** removes an individual override. Select **Direct** to set a zone's logical colour. OpenRGB profiles can store and restore the zone overrides. Older clients retain whole-device modes and zone colours.

SDK v6 sends accepted mode and colour changes to all connected v6 clients, including changes made through an older client. Older clients must refresh or reconnect to read changes made elsewhere. The AIO and other ordinary devices also share their last accepted SDK settings across connections; these are not readings of the physical LEDs. Initial colours come from saved zone settings where available. A shared saved effect supplies the initial device mode; different saved effects initialize independent zone modes where supported. Unspecified colours start black.

For ordinary devices, a mode change is attempted on every zone. If some zones reject it, the daemon logs the failures and reports the requested setting for the device; the zones may differ until a later successful write. If every zone rejects it, the reported mode remains unchanged.

Stored colour updates received while a hardware effect is selected update the SDK cache without replacing the effect with Static. Select Direct to send zone colours to the device.

## Profiles and reconnecting

Changing this setting reconnects OpenRGB and changes the affected devices' identities. Recreate profiles for those fan groups after changing the setting. Their v6 identity ends in `:regions`; older clients use `:inner` and `:outer`.

Lighting settings survive a client reconnect. SDK commands change the current session without overwriting native saved presets. After restarting the daemon or SDK server, the reported starting state comes from the effective native settings, including active presets. The hardware can retain the previous session's effect until another command is sent. Load an OpenRGB profile to restore that session's lighting.

Changing the available devices or their LED counts disconnects SDK clients so they can reconnect with the updated device list. Devices whose capabilities have not changed retain their session settings and pending writes. A removed fan group starts from native configuration if it is detected again. Reopened or invalidated devices with unchanged capabilities replay their accepted SDK settings, including ordinary devices such as the AIO. Unrelated wireless changes do not restart fan effects. This applies with or without the region setting enabled. The server supports protocol v6 for both layouts and negotiates an older version when required.

Idle SDK connections remain open. Once a client starts a packet, it must finish within ten seconds. Shutdown closes client sockets to interrupt idle reads. Zone write failures are logged once until recovery, and failed writes do not update the controller's successful-write cache. Driver acceptance may itself mean a command was queued during initialization; neither the SDK cache nor a successful driver call proves what the LEDs display.

The bridge keeps the latest complete update for each fan group and sends it through the native group-effect driver. Failed writes receive up to three attempts with increasing delays; sending the same setting again can retry an exhausted update. The first failure and retry exhaustion are logged, including ownership or policy refusals. An SDK state notification reports the accepted setting; it does not confirm that the LEDs have displayed it.

## Configuration and access

The setting is `rgb.openrgb_regions`, disabled by default.

Each connection has a bounded outgoing queue and one socket writer. A client that cannot keep up is disconnected so it can reconnect and read the current state; its socket writes do not block other clients.

The existing SDK listener has no authentication and binds all IPv4 interfaces. Use the host firewall to limit access if it should be local only. Run only one bridge controlling these devices.
