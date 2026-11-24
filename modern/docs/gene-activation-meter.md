# Gene activation meter

An open Gene Activations window watches the selected robot. The engine updates its percentages once per eligible DNA execution, including a console `execrob` at the same simulation cycle. Focus changes preserve the percentages for the same gene count. The next execution with a different count starts from zero.

## Conditioned `else` column numbering

The meter uses the VM’s current gene ordinals. Original Darwinbots 2.48.32 counts a standalone `else` after `start` as another column. With a false condition followed by `start … else … stop`, the original skips both bodies. A direct `cond … else` can run its body when the condition is false. The VM at this branch point groups the first example into one active column. Native execution and structural gene-count metadata are tracked separately. The source paths are [VB6 execution](../../Darwinbots2/DNA.bas), [VB6 gene counting](../../Darwinbots2/Module1.bas), and [VM execution](../engine/src/vm.rs).
