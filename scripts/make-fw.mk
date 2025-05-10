# --------------------------------------------------------------------------------
# make-fw: makefile frameworks for c/c++ projects
# https://github.com/fabrice-baray/make-fw
#


ifndef make-fw.included
make-fw.included:=1

# --------------------------------------------------------------------------------
# default settings

# build folder relative path to ROOT
make-fw.BUILD_FOLDER?=../_build

make-fw.LIB?=lib
make-fw.OBJ?=obj
make-fw.BIN?=bin
make-fw.DEP?=dep

# MODE of compilation, dbg, opt, ...
make-fw.dbgMODE?=dbg
make-fw.optMODE?=opt
make-fw.MODE?=$(make-fw.optMODE)

# default compilation flag
make-fw.default.CFLAGS?=-Wall
make-fw.default.dbgCFLAGS?=-g
make-fw.default.optCFLAGS?=-O3 -DNDEBUG

# --------------------------------------------------------------------------------
# GLOBAL variables to be used in src makefiles

# ROOT path to top of src folder
ROOT?=.

# OUT is the relative path to the output folder
make-fw.OBASE:=$(ROOT)/$(make-fw.BUILD_FOLDER)
OUT:=$(make-fw.OBASE)/$(make-fw.MODE)


# --------------------------------------------------------------------------------
# --------------------------------------------------------------------------------
# NOTHING below should be needed for src makefiles

# --------------------------------------------------------------------------------
# Object file dependencies management

# -MMD flag is to generate a dependency file at the same time of compilation
# -MP flag is to add a target for each prerequisite in the list, to avoid errors when
#     some flags are deleted
# generated file is with .Td extension (temporary), and then only after the
#   compilation is finished an mv command in the compilation rule moves it to
#   .d extension. This is to avoid corrupted file remaining if compilation failed
#   abruptly, or bad timestamp. In any case we do not return error if mv or touch
#   fails, the worst case dependency is missing and will be re-generated the next
#   compilation.
DEPFLAGS = -MT $@ -MMD -MP -MF $(OUT)/$(make-fw.DEP)/$*.Td
POSTCOMPILE = mv -f $(OUT)/$(make-fw.DEP)/$*.Td $(OUT)/$(make-fw.DEP)/$*.d && touch $@ | true

# .o target will be dependent of the dependencies file in case this last one is
# deleted. Then an empty fake rule is added for it to avoid a "no rule to make
# target"
$(OUT)/$(make-fw.DEP)/%.d: ;
.PRECIOUS: $(OUT)/$(make-fw.DEP)/%.d


# --------------------------------------------------------------------------------
# settings, global and per targets

#$(VERBOSE).SILENT:

CFLAGS+=$(make-fw.default.CFLAGS)
$(make-fw.OBASE)/$(make-fw.dbgMODE)/$(make-fw.OBJ)/%: CFLAGS+=$(make-fw.default.dbgCFLAGS)
$(make-fw.OBASE)/$(make-fw.optMODE)/$(make-fw.OBJ)/%: CFLAGS+=$(make-fw.default.optCFLAGS)

$(make-fw.OBASE)/$(make-fw.dbgMODE)/$(make-fw.BIN)/%: LDFLAGS+=-L $(make-fw.OBASE)/$(make-fw.dbgMODE)/$(make-fw.LIB) -Xlinker -R -Xlinker ../$(make-fw.LIB)
$(make-fw.OBASE)/$(make-fw.optMODE)/$(make-fw.BIN)/%: LDFLAGS+=-L $(make-fw.OBASE)/$(make-fw.optMODE)/$(make-fw.LIB) -Xlinker -R -Xlinker ../$(make-fw.LIB)


# --------------------------------------------------------------------------------
# generic compilation rules
#	mkdir -p #(shell dirname $@)
.SECONDEXPANSION:
.PHONY: clean

# order dependencies are obj and dep folders
$(OUT)/$(make-fw.OBJ)/%.o: $(ROOT)/%.c $(OUT)/$(make-fw.DEP)/%.d | $$(@D)/.folder $(OUT)/$(make-fw.DEP)/$$(dir $$(*)).folder
	$(COMPILE.c) $< $(DEPFLAGS) -o $@ ; $(POSTCOMPILE)

$(OUT)/$(make-fw.LIB)/%.so: | $(OUT)/.folders
	$(CC) $(CFLAGS) $(CPPFLAGS) $(LDFLAGS) -shared -o $@ $<

$(OUT)/$(make-fw.LIB)/%.a: | $(OUT)/.folders
	$(AR) $(ARFLAGS) $@ $^

$(OUT)/$(make-fw.BIN)/%: | $(OUT)/.folders
	$(LINK.s) $(filter-out %.so,$^)  $(LDLIBS) -o $@

all:
clean:
	rm -fr $(make-fw.OBASE)


# rules to create output folders
$(OUT)/.folders:
	mkdir -p $(OUT)/$(make-fw.BIN) $(OUT)/$(make-fw.LIB) $(OUT)/$(make-fw.DEP)
	touch $@

%/.folder: 
	mkdir -p $(@D)
	touch $@


# --------------------------------------------------------------------------------
# function helpers

# construct the list of object/dependency files: $(make-fw.{objects|deps} folder, list of src files)
# eg. $(make-fw.objects libX, file.c)
make-fw.objects=$(patsubst %.c,$(OUT)/$(make-fw.OBJ)/$(1)/%.o,$(2))
make-fw.deps=$(patsubst %.c,$(OUT)/$(make-fw.DEP)/$(1)/%.d,$(2))


else

# to allow makefile to not add to all
all.defined:=1

endif

