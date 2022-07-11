# Top-Level Tasks or Top-Level Loop

Since handle_message is its own task it can NOT
block the next execution.
Should this be default or should the join handle be
stored, and the next message receival be enqueued into a wait queue?
