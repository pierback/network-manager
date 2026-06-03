# Network Manager

Network Manager helps distinguish personally relevant devices from all devices visible on a home or private network.

## Language

**Discovered Device**:
A device automatically observed on a LAN or private overlay network.
_Avoid_: Raw device, network item, manually added device

**Discovery Observation**:
A time-bound piece of evidence that a device or endpoint was observed.
_Avoid_: Device identity, current state

**Device Identity**:
The stable representation of a device across changing reachable addresses and names.
_Avoid_: Endpoint, address, hostname

**Device Label**:
A user-chosen display name for a device.
_Avoid_: Hostname, DNS name, alias

**Device Alias**:
A unique CLI-friendly name for a device.
_Avoid_: Device label, hostname, DNS name

**Device Category**:
A weakly inferred or user-corrected kind of device.
_Avoid_: Identity, label

**Device Tag**:
A user-facing grouping marker for organizing devices.
_Avoid_: Identity evidence, category

**Tracked Device**:
A device identity intentionally selected by the user as relevant enough to appear in the dashboard.
_Avoid_: Favorite, pinned device

**Ignored Device**:
A device identity intentionally excluded from normal discovery attention.
_Avoid_: Deleted device, untracked device

**Dashboard**:
The primary view that shows the user's tracked devices and their current availability.
_Avoid_: Full device list, scan results

**Quick Access View**:
A compact entry point for checking tracked devices and acting on their endpoints.
_Avoid_: Dashboard, discovery view

**Discovery View**:
The view for browsing discovered devices and selecting which identities should be tracked.
_Avoid_: Dashboard, quick access view

**Network Endpoint**:
A reachable address or name associated with a device.
_Avoid_: Address, host, connection target

**SSH Target**:
The network endpoint selected as the best destination for opening an SSH session to a device.
_Avoid_: Device identity, device label

**Endpoint Preference**:
A policy for choosing which network endpoint should be used first for actions such as SSH.
_Avoid_: Reachability, availability

**SSH Capability**:
Whether a network endpoint appears suitable for opening an SSH session.
_Avoid_: Endpoint reachability, device availability

**Network Proximity**:
The current evidence that a device is reachable through the local network path from this Mac.
_Avoid_: Home network, same Wi-Fi, subnet

**Tailscale Presence**:
A device's known membership or visibility in the user's Tailscale network.
_Avoid_: LAN presence, local service status

**Endpoint Reachability**:
Whether a network endpoint appears usable right now.
_Avoid_: Device identity, Tailscale service status

**Availability State**:
The current confidence that a service, device presence, or endpoint is available: online, offline, or unknown.
_Avoid_: Color, health, stale data

**Last Seen**:
The most recent time a device identity or endpoint was observed.
_Avoid_: Offline time, discovery time

**Discovery Scope**:
The set of devices observable from the Mac running the app.
_Avoid_: Entire network, all possible devices

**Discovery Source**:
The mechanism or network surface through which a device or endpoint was observed.
_Avoid_: Device identity, endpoint type

**mDNS SSH Service**:
A Bonjour-advertised `_ssh._tcp` service observed on the local network.
_Avoid_: Device identity, guaranteed SSH availability

**Portable User Settings**:
The exportable user-owned layer: labels, aliases, tracked state, categories, tags, SSH settings, endpoint preferences, and identity corrections.
_Avoid_: Discovery cache, scan results

**Network Interface**:
A local network path through which devices or endpoints can be observed.
_Avoid_: Device identity, endpoint

**Identity Evidence**:
Information used to decide whether observations belong to the same device identity.
_Avoid_: User correction, label

**Identity Correction**:
A user decision that corrects whether discoveries belong to the same device identity or separate identities.
_Avoid_: Rename, label

## Relationships

- A **Discovery Observation** provides evidence for a **Discovered Device** or **Network Endpoint**.
- A **Discovered Device** resolves to one **Device Identity** when enough identifying information is available.
- Multiple **Discovered Devices** may resolve to the same **Device Identity**.
- A **Discovery Observation** has one **Discovery Source**.
- A **Discovery Observation** may be associated with one **Network Interface**.
- **Identity Evidence** informs automatic resolution of **Discovered Devices** into **Device Identities**.
- An **Identity Correction** can override automatic resolution of **Discovered Devices** into **Device Identities**.
- A **Discovered Device** may become a **Tracked Device**.
- A **Discovered Device** may become an **Ignored Device**.
- A **Tracked Device** appears in the **Dashboard**.
- A **Tracked Device** may appear in the **Quick Access View**.
- A **Tracked Device** may have one **Device Label**.
- A **Tracked Device** may have one **Device Alias**.
- A **Tracked Device** cannot also be an **Ignored Device**.
- A **Device Identity** may have one **Device Category**.
- A **Device Identity** may have zero or more **Device Tags**.
- The **Discovery View** shows **Discovered Devices** and **Device Identities** whether or not they are tracked.
- A **Tracked Device** has one or more **Network Endpoints**.
- An **SSH Target** is chosen from a device's **Network Endpoints**.
- A **Network Endpoint** may have **SSH Capability**.
- **Network Proximity** and **Endpoint Preference** influence which **Network Endpoint** becomes the preferred **SSH Target**.
- A **Network Endpoint** may have an **Endpoint Reachability** state.
- A **Device Identity** may have **Tailscale Presence**.
- **Tailscale Presence**, **Endpoint Reachability**, and **SSH Capability** each have an **Availability State**.
- **Last Seen** can apply to a **Device Identity** or **Network Endpoint**.
- **Portable User Settings** apply to matching discovered identities; importing them must not create manual devices that were never discovered.
- The user daemon exposes typed local IPC for reads, SSH resolution, refresh requests, and user-intent mutations.
- The user daemon performs automatic bounded quick refreshes so discovery and endpoint status can stay warm without the UI or CLI polling manually.

## Example dialogue

> **Dev:** "Should the dashboard show every **Discovered Device**?"
> **Domain expert:** "No — it should focus on **Tracked Devices**, while still allowing discovery of new devices."

## Flagged ambiguities

- "favorites" was used for intentionally visible devices — resolved: the domain term is **Tracked Device**; UI copy may still use "Favorites" if useful.
- "identity" distinguishes a device itself from its changing **Network Endpoints**.
- "rename" means assigning a **Device Label**, not modifying discovered hostnames or DNS names.
- "available through Tailscale" is not the same as a LAN address; it refers to **Tailscale Presence** and endpoint-specific **Endpoint Reachability**.
- "online" and "offline" must be scoped to a specific status: local Tailscale service, **Tailscale Presence**, or **Endpoint Reachability**.
- Red/green UI language maps to **Availability State**, but unknown/stale/error must remain distinct from offline.
- A stale status is treated as unknown, not offline.
- Manual device entry was rejected — devices should originate as **Discovered Devices**, then be selected as **Tracked Devices**.
- "all devices" means devices inside the **Discovery Scope**, not every device that exists on the physical network.
- Automatic identity merging is acceptable only if **Identity Correction** can undo wrong merges or splits.
- "home network" was resolved to **Network Proximity**: prefer LAN endpoints when local reachability is proven, otherwise prefer Tailscale endpoints.
- CLI lookup may use a **Device Label** or discovered names, but ambiguous matches must be disambiguated instead of guessed.
- "device type" was resolved to **Device Category** because inferred categories are imperfect and user-correctable.
- Being reachable is distinct from **SSH Capability**; a device can be online without being a useful SSH target.
- A **Device Alias** is for unambiguous CLI lookup; a **Device Label** is for human-facing display.
- "delete discovered device" was resolved to **Ignored Device**, because observable devices may be rediscovered.
- User-provided **Identity Correction** overrides automatic **Identity Evidence**, even when new evidence conflicts.
- **Device Tags** are for user organization only; they are not identity evidence.
- IP addresses are **Network Endpoints**, not **Device Identities**; LAN IP alone must not identify a device.
- Hostnames are **Identity Evidence**, but not absolute proof of identity.
- Bonjour/mDNS `_ssh._tcp` observations are a **Discovery Source** for local SSH endpoints; SSH probing still decides **SSH Capability**.
- Export/import should preserve user intent and **Identity Corrections**, but not volatile discovery observations or endpoint reachability.
