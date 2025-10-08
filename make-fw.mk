# --------------------------------------------------------------------------------
# make-fw: makefile frameworks for c/c++ projects
# https://github.com/fabrice-baray/make-fw
#
# SPDX-FileCopyrightText: Fabrice Baray
# SPDX-License-Identifier: MIT


ifndef mfwIncluded
mfwIncluded:=1

# --------------------------------------------------------------------------------
# default settings

# current path
mfwPATH:=$(shell pwd)
mfwRELPATH=$(shell realpath -s --relative-to="$(mfwPATH)/$(ROOT)" "$(mfwPATH)")

# build folder relative path to ROOT
mfwBUILD_FOLDER?=../_build

mfwLIB?=lib
mfwOBJ?=obj
mfwBIN?=bin
mfwDEP?=dep

# MODE of compilation, dbg, opt, ...
MODE?=$(mfwMODE)
mfwMODE?=$(mfwOPT)
mfwDBG?=dbg
mfwOPT?=opt
mfwASAN?=asan

ifeq (,$(filter $(mfwDBG) $(mfwOPT) $(mfwASAN),$(MODE)))
  $(error MODE should be either $(mfwDBG), $(mfwOPT) or $(mfwASAN))
endif

# default compilation flag
mfwCFLAGS?=-Wall -I$(ROOT)
mfw$(mfwDBG)CFLAGS?=-g
mfw$(mfwOPT)CFLAGS?=-O3 -DNDEBUG
mfw$(mfwASAN)CFLAGS?=-O1 -g -fsanitize=address

mfwCXXFLAGS?=-Wall -I$(ROOT)
mfw$(mfwDBG)CXXFLAGS?=-g
mfw$(mfwOPT)CXXFLAGS?=-O3 -DNDEBUG
mfw$(mfwASAN)CXXFLAGS?=-O1 -g -fsanitize=address

mfw$(mfwASAN)LDFLAG=-fsanitize=address

# --------------------------------------------------------------------------------
# GLOBAL variables to be used in src makefiles

# ROOT path to top of src folder
ROOT?=.

# OUT is the relative path to the output folder
mfwOBASE:=$(ROOT)/$(mfwBUILD_FOLDER)
OUT:=$(mfwOBASE)/$(MODE)


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
mfwPOSTCOMPILE = mv -f $(OUT)/$(mfwDEP)/$*.Td $(OUT)/$(mfwDEP)/$*.d && touch -c $@ | true

# .o target will be dependent of the dependencies file in case this last one is
# deleted. Then an empty fake rule is added for it to avoid a "no rule to make
# target"
$(OUT)/$(mfwDEP)/%.d: ;
.PRECIOUS: $(OUT)/$(mfwDEP)/%.d


# --------------------------------------------------------------------------------
# settings, global and per targets

CFLAGS+=$(mfwCFLAGS)
$(mfwOBASE)/$(mfwDBG)/$(mfwOBJ)/%: CFLAGS+=$(mfw$(mfwDBG)CFLAGS)
$(mfwOBASE)/$(mfwOPT)/$(mfwOBJ)/%: CFLAGS+=$(mfw$(mfwOPT)CFLAGS)
$(mfwOBASE)/$(mfwASAN)/$(mfwOBJ)/%: CFLAGS+=$(mfw$(mfwASAN)CFLAGS)

CXXFLAGS+=$(mfwCXXFLAGS)
$(mfwOBASE)/$(mfwDBG)/$(mfwOBJ)/%: CXXFLAGS+=$(mfw$(mfwDBG)CXXFLAGS)
$(mfwOBASE)/$(mfwOPT)/$(mfwOBJ)/%: CXXFLAGS+=$(mfw$(mfwOPT)CXXFLAGS)
$(mfwOBASE)/$(mfwASAN)/$(mfwOBJ)/%: CXXFLAGS+=$(mfw$(mfwASAN)CXXFLAGS)

$(mfwOBASE)/$(mfwDBG)/$(mfwBIN)/%: LDFLAGS+=-L $(mfwOBASE)/$(mfwDBG)/$(mfwLIB) -Xlinker -R -Xlinker '$$ORIGIN'/../$(mfwLIB)
$(mfwOBASE)/$(mfwOPT)/$(mfwBIN)/%: LDFLAGS+=-L $(mfwOBASE)/$(mfwOPT)/$(mfwLIB) -Xlinker -R -Xlinker '$$ORIGIN'/../$(mfwLIB)
$(mfwOBASE)/$(mfwASAN)/$(mfwBIN)/%: LDFLAGS+=-L $(mfwOBASE)/$(mfwASAN)/$(mfwLIB) -Xlinker -R -Xlinker '$$ORIGIN'/../$(mfwLIB) $(mfw$(mfwASAN)LDFLAG)


# --------------------------------------------------------------------------------
# generic compilation rules
.SECONDEXPANSION:
.PHONY: clean cleanR mrproper

# order dependencies are obj and dep folders
$(OUT)/$(mfwOBJ)/%.o: $(ROOT)/%.c $(OUT)/$(mfwDEP)/%.d | $$(@D)/.folder $(OUT)/$(mfwDEP)/$$(dir $$(*)).folder
	@echo "[cc]" $(patsubst $(patsubst ./%,%,$(mfwOBASE))/%,%,$@)
	$(COMPILE.c) $< $(mfwDEPFLAGS) -o $@ ; $(mfwPOSTCOMPILE)

$(OUT)/$(mfwOBJ)/%.o: $(ROOT)/%.cc $(OUT)/$(mfwDEP)/%.d | $$(@D)/.folder $(OUT)/$(mfwDEP)/$$(dir $$(*)).folder
	@echo "[c+]" $(patsubst $(patsubst ./%,%,$(mfwOBASE))/%,%,$@)
	$(COMPILE.cc) $< $(mfwDEPFLAGS) -o $@ ; $(mfwPOSTCOMPILE)

$(OUT)/$(mfwOBJ)/%.o: $(ROOT)/%.cpp $(OUT)/$(mfwDEP)/%.d | $$(@D)/.folder $(OUT)/$(mfwDEP)/$$(dir $$(*)).folder
	@echo "[c+]" $(patsubst $(patsubst ./%,%,$(mfwOBASE))/%,%,$@)
	$(COMPILE.cc) $< $(mfwDEPFLAGS) -o $@ ; $(mfwPOSTCOMPILE)

$(OUT)/$(mfwLIB)/%.so: | $(OUT)/.folders
	@echo "[ld]" $(patsubst $(patsubst ./%,%,$(mfwOBASE))/%,%,$@)
	$(LINK.cc) $(LDFLAGS) -shared -o $@ $<

$(OUT)/$(mfwLIB)/%.a: | $(OUT)/.folders
	@echo "[ar]" $(patsubst $(patsubst ./%,%,$(mfwOBASE))/%,%,$@)
	$(AR) $(ARFLAGS) $@ $^

$(OUT)/$(mfwBIN)/%: | $(OUT)/.folders
	@echo "[ld]" $(patsubst $(patsubst ./%,%,$(mfwOBASE))/%,%,$@)
	$(LINK.cc) $(filter-out %.so,$^) $(LDLIBS) -o $@

all:

ifndef mfwCLEAN
mfwCLEAN=1
clean:
	cd $(mfwOBASE) ; F=$$(find $(MODE)/$(mfwOBJ)/$(mfwRELPATH)/ -maxdepth 1 -type f ) ; if [ ! -z "$$F" ] ; then echo rm -f $$F ; rm -f $$F ; fi

cleanR:
	echo rm -fr $(MODE)/$(mfwOBJ)/$(mfwRELPATH)/* ; rm -fr $(OUT)/$(mfwOBJ)/$(mfwRELPATH)/*

endif

mrproper:
	@echo rm -fr $(mfwOBASE)
	rm -fr $(mfwOBASE)

$(VERBOSE).SILENT:

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
mfwOBJECTS=$(patsubst %.c,$(OUT)/$(mfwOBJ)/$(1)/%.o,$(filter %.c,$(2))) $(patsubst %.cc,$(OUT)/$(mfwOBJ)/$(1)/%.o,$(filter %.cc,$(2))) $(patsubst %.cpp,$(OUT)/$(mfwOBJ)/$(1)/%.o,$(filter %.cpp,$(2)))
mfwDEPS=$(patsubst %.c,$(OUT)/$(mfwDEP)/$(1)/%.d,$(filter %.c,$(2))) $(patsubst %.cc,$(OUT)/$(mfwDEP)/$(1)/%.d,$(filter %.cc,$(2))) $(patsubst %.cpp,$(OUT)/$(mfwDEP)/$(1)/%.d,$(filter %.cpp,$(2)))


else

# to allow makefile to not add to all
all.defined:=1

endif

