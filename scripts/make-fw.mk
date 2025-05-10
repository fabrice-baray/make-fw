# --------------------------------------------------------------------------------
# make-fw: makefile frameworks for c/c++ projects
# https://github.com/fabrice-baray/make-fw
#


ifndef mfwIncluded
mfwIncluded:=1

# --------------------------------------------------------------------------------
# default settings

# build folder relative path to ROOT
mfwBUILD_FOLDER?=../_build

mfwLIB?=lib
mfwOBJ?=obj
mfwBIN?=bin
mfwDEP?=dep

# MODE of compilation, dbg, opt, ...
mfwdbgMODE?=dbg
mfwoptMODE?=opt
mfwMODE?=$(mfwoptMODE)

# default compilation flag
mfwdefault_CFLAGS?=-Wall
mfwdefault_dbgCFLAGS?=-g
mfwdefault_optCFLAGS?=-O3 -DNDEBUG

# --------------------------------------------------------------------------------
# GLOBAL variables to be used in src makefiles

# ROOT path to top of src folder
ROOT?=.

# OUT is the relative path to the output folder
mfwOBASE:=$(ROOT)/$(mfwBUILD_FOLDER)
OUT:=$(mfwOBASE)/$(mfwMODE)


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
mfwDEPFLAGS = -MT $@ -MMD -MP -MF $(OUT)/$(mfwDEP)/$*.Td
mfwPOSTCOMPILE = mv -f $(OUT)/$(mfwDEP)/$*.Td $(OUT)/$(mfwDEP)/$*.d && touch $@ | true

# .o target will be dependent of the dependencies file in case this last one is
# deleted. Then an empty fake rule is added for it to avoid a "no rule to make
# target"
$(OUT)/$(mfwDEP)/%.d: ;
.PRECIOUS: $(OUT)/$(mfwDEP)/%.d


# --------------------------------------------------------------------------------
# settings, global and per targets

#$(VERBOSE).SILENT:

CFLAGS+=$(mfwdefault_CFLAGS)
$(mfwOBASE)/$(mfwdbgMODE)/$(mfwOBJ)/%: CFLAGS+=$(mfwdefault_dbgCFLAGS)
$(mfwOBASE)/$(mfwoptMODE)/$(mfwOBJ)/%: CFLAGS+=$(mfwdefault_optCFLAGS)

$(mfwOBASE)/$(mfwdbgMODE)/$(mfwBIN)/%: LDFLAGS+=-L $(mfwOBASE)/$(mfwdbgMODE)/$(mfwLIB) -Xlinker -R -Xlinker ../$(mfwLIB)
$(mfwOBASE)/$(mfwoptMODE)/$(mfwBIN)/%: LDFLAGS+=-L $(mfwOBASE)/$(mfwoptMODE)/$(mfwLIB) -Xlinker -R -Xlinker ../$(mfwLIB)


# --------------------------------------------------------------------------------
# generic compilation rules
#	mkdir -p #(shell dirname $@)
.SECONDEXPANSION:
.PHONY: clean

# order dependencies are obj and dep folders
$(OUT)/$(mfwOBJ)/%.o: $(ROOT)/%.c $(OUT)/$(mfwDEP)/%.d | $$(@D)/.folder $(OUT)/$(mfwDEP)/$$(dir $$(*)).folder
	$(COMPILE.c) $< $(mfwDEPFLAGS) -o $@ ; $(mfwPOSTCOMPILE)

$(OUT)/$(mfwLIB)/%.so: | $(OUT)/.folders
	$(CC) $(CFLAGS) $(CPPFLAGS) $(LDFLAGS) -shared -o $@ $<

$(OUT)/$(mfwLIB)/%.a: | $(OUT)/.folders
	$(AR) $(ARFLAGS) $@ $^

$(OUT)/$(mfwBIN)/%: | $(OUT)/.folders
	$(LINK.s) $(filter-out %.so,$^)  $(LDLIBS) -o $@

all:
clean:
	rm -fr $(mfwOBASE)


# rules to create output folders
$(OUT)/.folders:
	mkdir -p $(OUT)/$(mfwBIN) $(OUT)/$(mfwLIB) $(OUT)/$(mfwDEP)
	touch $@

%/.folder: 
	mkdir -p $(@D)
	touch $@


# --------------------------------------------------------------------------------
# function helpers

# construct the list of object/dependency files: $(mfw{OBJECTS|DEPS} folder, list of src files)
# eg. $(mfwOBJECTS libX, file.c)
mfwOBJECTS=$(patsubst %.c,$(OUT)/$(mfwOBJ)/$(1)/%.o,$(2))
mfwDEPS=$(patsubst %.c,$(OUT)/$(mfwDEP)/$(1)/%.d,$(2))


else

# to allow makefile to not add to all
all.defined:=1

endif

