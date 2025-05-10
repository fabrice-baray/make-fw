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

- use as much as possible target specific variables (keep locality)

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

- make-fw file can be install in base_folder or base_folder/externals as an external dependency
- \<mode\> corresponds to compilation modes, optimized (opt), debug (dbg), asan (asan),...
- test folder can be used as an example of make-fw usage

## Rules to write makefiles

**1. prevent double inclusion** 

    ifndef file_is_included
    file_is_included:=1
      ...
    endif


**2. define ROOT and include make-fw.mk from that reference**
  - ROOT is the relative path to the root of the source tree (src folder in the tree example).
  - OUT variable pointing to the output build folder is defined by make-fw (_build folder in the tree example).
  - you can include your own .mk file defining new implicit rules

        ROOT?=..
        include $(ROOT)/../make-fw.mk

**3. always include generated dependency files**

        -include $(OUT)/dep/<folder>/file.d
		
**4. define your own dependencies with:**

  - create a static library with some object files:

        $(OUT)/lib/libA.a: $(OUT)/obj/libA/file.o

  - optionally define an *all:* rule for what you want to compile by default in that folder:
  
        ifndef all.defined
          all: $(OUT)/lib/libA.a
        endif

  - create a binary linking some object files
  
        $(OUT)/bin/tool: $(OUT)/obj/tool/file.o

  - add a dependency between a tool and a static library:

        include $(ROOT)/lib/makefile
        $(OUT)/bin/tool: LDFLAGS+=$(OUT)/lib/lib.a
        $(OUT)/bin/tool: $(OUT)/lib/lib.a
        
  - create a dynamic library with some object files:

        $(OUT)/lib/libB.so: $(OUT)/obj/libB/file.o
        $(OUT)/obj/libB/file.o: CFMAGS+=-shared

  - add a dependency between a tool and a dynamic library:
  
        $(OUT)/bin/tool: $(OUT)/lib/libX.so
		$(OUT)/bin/tool: LDLIBS+=-lX
		
		
## Makefile usage

- compilation can be invoked just by calling ``make`` in any folder
- compilation with a different mode: ``make MODE=dbg``

## Makefile configuration defaults

You can change several default settings by assigning those variables before
including make-fw.mk:

- output build folder pathname: **mfwBUILD_FOLDER**  
  defaults to **../_build**
  
- output folder names: **mfwOBJ**, **mfwDEP**, **mfwLIB**, **mfwBIN**  
  defaults to **obj**, **dep**, **lib**, **bin**
  
- mode names: **mfwDBG**, **mfwOPT**  
  defaults to **dbg**, **opt**
  
- default selected mode: **mfwMODE**__
  defaults to **$(mfwOPT)**
  
- default CFLAGS: **mfwCFLAGS**  
  defaults to **-Wall**
  
- default CFLAGS per mode: **mfw\<mode\>CFLAGS**  
  defaults to:
  - dbg mode: **-g**
  - opt mode: **-O3 -DNDEBUG**

