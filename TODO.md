### Issues

1) Current unwind behaviour is inconsistent between sync and async 
   -> will never be simple as long as this distinction exist

2) Runtime/Builder is never used for other cases than Sim -> remove complexity

3) Tokio Time integration

### Solutions

- needs tokio PR

### Idea: Sync Points

Certain distributed algorithms like routing require some kind of synchronization
this can be archived using at_sim_start-stages but not with any other way
=> sync points

- Global register to match same group
- local: `SyncPoint::(ID, CFG).await`
- global: List of wakers
- trigger: CFG dependent: manual `SyncPoint::trigger` or global-timeout (in cfg)

To be learned:
- calling wakers of inactive runtimes
