# Top-Level Tasks or Top-Level Loop

Since handle_message is its own task it can NOT
block the next execution.
Should this be default or should the join handle be
stored, and the next message receival be enqueued into a wait queue?

# Time Wakeup handleing

-   Global waker with custom events more efficient, BUT
    mutiple modules may be waken up at the same time
    thus mutltiple "runtimes" may be active at the
    same time (shouldnt be a problem BUT mhh)
