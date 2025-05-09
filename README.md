# make-fw
Makefile framework

make-fw aims to provide a simple framework to use makefile for project
compilations. Even though makefiles are an old way to handle project
compilations, it is very stable, has most of the needed feature, is largely
known by developers and so is still a very reliable tool to be
considered. However to avoid some pitfalls, complicated syntax and some
nightmares, I felt that developing a framework could be useful. I tried to
respect the following principles:

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

- for simplicity, avoid as much as possible the user of $(eval ...) function


Other references:
-  gnu make manual: https://www.gnu.org/software/make/manual/html_node/index.html
-  http://make.mad-scientist.net/papers/multi-architecture-builds
