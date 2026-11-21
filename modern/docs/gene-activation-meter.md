# Gene activation meter

An open Gene Activations window watches the selected robot. The engine updates its percentages once per eligible DNA execution, including a console `execrob` at the same simulation cycle. Focus changes preserve the percentages for the same gene count. The next execution with a different count starts from zero.

## Conditioned `else` column numbering

The meter uses the VM’s current gene ordinals. Original Darwinbots 2.48.32 gives an adjacent conditioned `else` its own column. For example, `cond 0 1 > start 1 .up store else 1 .dn store stop` shows two columns and activates column 2 in the original. The VM at this branch point groups these arms as one column. The source paths are [VB6 execution](../../Darwinbots2/DNA.bas), [VB6 gene counting](../../Darwinbots2/Module1.bas), and [VM execution](../engine/src/vm.rs).
