# make-fw - a simple Makefile framework

make-fw aims to provide a simple framework to use makefile for project
compilations (originally C/C++ projects). Even though makefiles are an old way
to handle project compilations, it is very stable, has most of the needed
features, is largely known by developers and so is still a very reliable tool to
be considered. However to avoid some pitfalls, complicated syntaxes and
recurring nightmares, I felt that having a guiding framework could actually be
something useful. I tried to respect the following principles:

- do not use recursive make invocations, instead a single invocation of make
  should construct the whole dependency tree (following the idea presented by
  Peter Miller in the paper "Recursive Make Considered Harmfuf"
  https://accu.org/journals/overload/14/71/miller_2004/)

- keep structured source directories, for libraries and tools

- being able to run make from any level of the source directories

- generate all output files outside of the source tree, allow separated folders
  for compilations modes, like debug, optimize, asan, ...

- auto generation of include dependencies
  (http://make.mad-scientist.net/papers/advanced-auto-dependency-generation)

- for simplicity, avoid as much as possible the use of $(eval ...) function

- ...

Other references:
-  gnu make manual: https://www.gnu.org/software/make/manual/html_node/index.html
-  http://make.mad-scientist.net/papers/multi-architecture-builds




## Project global structure


```md
base_folder
├── src
│   ├── makefile
│   ├── libA
│   |   ├── makefile
│   |   └── file.c
│   ├── libB
│   |   └── makefile
│   └── toolA
│       └── makefile
└── _build
    └──<mode>
       ├── bin
       |   └── toolA
       ├── dep
       |   └── libA
       |       └── file.d
       ├── lib
       |   ├── libA.a
       |   └── libB.a
       └── obj
           ├── libA
           |   └── file.o
           ├── libB
           └── toolA
```

## Rules to write makefiles

### prevent double inclusion
```md
ifndef file_is_included
file_is_included:=1
...
endif
```

### define ROOT and include make-fw.mk from that reference
- ROOT is the relative path to the root of the source tree (src folder in the tree example).
- OUT variable pointing to the output build folder is defined by make-fw (_build folder in the tree example).

```md
ROOT?=..
include $(ROOT)/../scripts/make-fw.mk
```

### define your own dependency rules

- create a static library with some object files:
```md
$(OUT)/lib/libA.a: $(OUT)/obj/libA/file.o
```
