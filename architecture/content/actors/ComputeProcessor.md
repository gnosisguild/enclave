---
type: actor
description: Runs expensive jobs that require a threadpool to execute
tags:
  - todo
  - trbfv
  - compute
---
## `=this.file.name`

`=this.description`

---

Responsible for running slow multithreaded computations using thread parallelization systems such as rayon.

This actor can be run ins a SyncArbiter with perhaps 2 threads in order to manage two concurrent jobs. Within the actor when running a rayon enabled task ensure the handler 

We need to prepare our threadpool on startup so that rayon is given the correct number of threads.

Considering `n` Threads for SyncArbiter so we can handle multiple rayon jobs simultaneously and a rayon pool of "available_parallelism" - `n-4`

We may want to run the DataStore actor on a separate thread.

This aught to leave 1 thread for the normal actor model.

| Thread | Usage               |
| ------ | ------------------- |
| 1      | Actor               |
| 2      | Data Writing        |
| 3      | ThreadPoolCompute 1 |
| 4      | ThreadPoolCompute 2 |
| 5 - n  | Rayon Thread Pool   |
We can use `thread_pool.install(|| ...)` to include the ThreadPoolCompute thread in the rayon pool while the calculation is completing.