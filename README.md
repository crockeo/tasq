# tasq

Task management software based around graphs.

# Huh?

You can think of "things you want to do" as a directed acyclic graph (DAG).
Using the [Getting Things Done](https://gettingthingsdone.com/) (GTD) interpretation of task management
you can conceptualize nodes with different properties or different minimum depths
as different kinds of things:

* Nodes with 0 parents and >=1 child "areas of focus."
* Nodes with >=1 parent and >=1 child are "projects."
* Nodes with >=1 parent and 0 children are "tasks."

This kind of logic exists in most any GTD system
([Things](https://culturedcode.com/things/) epitomizes this approach, IMO).
A DAG-based approach is nice because you can ask the question "what should I work on next".
You can perform a DFS on any node,
find the nodes without children,
and then report them as tasks you can execute.

# Usage

## Installation

```shell
git clone git@github.com:crockeo/tasq
cd tasq
cargo install .
```

## Usage

```shell
# Good help page :)
tasq --help
```

# License

MIT Open Source License. See [LICENSE](./LICENSE).
