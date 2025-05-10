# --------------------------------------------------------------------------------
# 

# References:
#   https://www.gnu.org/software/make/manual/html_node/index.html
#   http://make.mad-scientist.net/papers/multi-architecture-builds
#   http://make.mad-scientist.net/papers/advanced-auto-dependency-generation
#   paper: Recursive Make Considered Harmful (Peter Miller)

ifndef make-fw.included
make-fw.included:=1

# --------------------------------------------------------------------------------
# output location
ROOT?=.
BUILD_FOLDER?=../_build
MODE?=opt

OBASE:=$(ROOT)/$(BUILD_FOLDER)
OUT:=$(OBASE)/$(MODE)

# --------------------------------------------------------------------------------
# settings

#$(VERBOSE).SILENT:

CFLAGS+=-Wall
$(OBASE)/dbg/obj/%: CFLAGS+=-g
$(OBASE)/opt/obj/%: CFLAGS+=-O3 -DNDEBUG

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
DEPFLAGS = -MT $@ -MMD -MP -MF $(OUT)/dep/$*.Td
POSTCOMPILE = mv -f $(OUT)/dep/$*.Td $(OUT)/dep/$*.d && touch $@ | true

# .o target will be dependent of the dependencies file in case this last one is
# deleted. Then an empty fake rule is added for it to avoid a "no rule to make
# target"
$(OUT)/dep/%.d: ;
.PRECIOUS: $(OUT)/dep/%.d


# --------------------------------------------------------------------------------
# generic compilation rules
#	mkdir -p #(shell dirname $@)
.SECONDEXPANSION:
.PHONY: clean

# order dependencies are obj and dep folders
$(OUT)/obj/%.o: $(ROOT)/%.c $(OUT)/dep/%.d | $$(@D)/.folder $(OUT)/dep/$$(dir $$(*)).folder
	$(COMPILE.c) $< $(DEPFLAGS) -o $@ ; $(POSTCOMPILE)


$(OUT)/lib/%.a: | $(OUT)/.folders
	$(AR) $(ARFLAGS) $@ $^

$(OUT)/bin/%: | $(OUT)/.folders
	$(LINK.s) $^ $(LDLIBS) -o $@

all:
clean:
	rm -fr $(OBASE)


# rules to create output folders
$(OUT)/.folders:
	mkdir -p $(OUT)/bin $(OUT)/lib $(OUT)/dep
	touch $@

%/.folder: 
	mkdir -p $(@D)
	touch $@


# --------------------------------------------------------------------------------
# function helpers

## adding a static library pre-requisite to a target, arguments:
##   1: target
##   2: pre-requisite
## it supposes that:
##   <target>_TARGET contains the name of the target
##   <target>_EXT_DEPS will contain external dependencies needed to link
#define add_ar_dep=
#include $(ROOT)/$(2)/makefile
#$(1): LDFLAGS+=$($(2).EXT_DEPS)
#$(1): $($(2).TARGET)
#endef

else

# to allow makefile to not add to all
all.defined:=1

endif

