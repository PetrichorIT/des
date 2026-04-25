Changelog
===

> since v.6.1

## v6.2

- **des**
  - added Channel v2 impl (T: Channel)
  - simplified Message constructors (body access)
  - removed ModuleExt

## v6.4

- **des**
  - removed Builder/Runtime as detached API from Sim
  - removed feature gate "net"
  - added GateClusters (+automatic)
  - changed to internals to des_sync 
  - added statisitics
  - attached ExecContext to current()
- **des-ndl**
  - moved NDL implemenation to extra crate
