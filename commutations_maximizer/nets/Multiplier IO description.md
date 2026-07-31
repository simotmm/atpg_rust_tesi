**Multiplier**

Tables 1 and 2 present the description, size, direction, and constraints for the multiplier parameters and signals from Klessydra-T. Table 3 is an example of a configuration to enable 32-bit operations by giving fixed values to the inputs except for the data input.

| Name | Value | Description |
| :---- | :---- | :---- |
| multithread\_accl\_en | 0 | Enable the multithreading inside the multiplier. It means allowing threads using the multiplier from other threads. Considered disabled.  |
| replicate\_accl\_en | 0, 1 | Enable the replication of the accelerator for each thread |
| SIMD | 1, 2, 4, 8 | Number of multiplier unit replication per accelerator.  Configuration-dependent parameter. |
| ACCL\_NUM | 1, 3 | Number of accelerators. If the value is equal to 3 *replicate\_accl\_en* should be equal to 1 |
| FU\_NUM | *ACCL\_NUM* | Number of *FU* replication. Each *FU* is composed of *SIMD* multipliers. |
| Data\_width | 32 | Size of the data |
| SIMD\_width | 32 \* *SIMD* | Size of the data for all the multipliers in one *FU* |

**Table 1: Multiplier parameters**

| Name | Direction | Size | Description | Constraints |
| ----- | ----- | ----- | ----- | ----- |
| clk\_i | Input | 1 | Clock |  |
| rst\_ni | Input | 1 | Reset. It should be high to enable operation. |  |
| FUNCT\_SELECT\_MASK | Input | 32 \* *ACCL\_NUM* | Enable intermediate adders, used when 32-bits operation is enabled | **For each accelerator:**  All 32 bits should be **‘0’** or **‘1**’. For example, with 3 accelerators, the first 32 bits can be all **‘1’**, the following 32 bits can be all **‘0’**, and the final 32 bits can be all **‘1’**. Bits are all **‘1’** if *MVTYPE\_DSP* of the corresponding accelerator is equal to **‘10’**. It should keep the values for at least two clocks. Changes when *mul\_en* goes to **high** state. Keep the values at least for 2 clocks after *mul\_en* goes to **low** state. |
|  MVTYPE\_DSP | Input | 2 \* *ACCL\_NUM* | Type of the operation (i.i., 8-bits, 16-bits, and 32-bits) | **For each accelerator:** It can assume the value of **‘00**’, **‘01’**, or **‘10’**. Keep the values for all the operations. |
| recover\_state\_wires | Input | *ACCL\_NUM* | Used to extend the write-back state when the FU halts | Should be in **high** state if *halt\_dsp\_lat* is in **high** state. Can goes to **high** state only if *mul\_stage\_2\_en* is in **high** state. |
| halt\_dsp\_lat | Input | *ACCL\_NUM* | Halt the operation when the memory can’t be written | Goes to **high** state one cycle after *recover\_state\_wires* goes to **high** state. Can goes to **high** state only if *mul\_stage\_2\_en* was in **high** state one cycle before. If it goes to a **high** state should stay at least 2 more clock cycles. |
| mul\_stage\_1\_en | Input | *ACCL\_NUM* | Enable the first stage of the multiplier if data is ready. Stays high if new data should be processed by the multiplier. | Goes to **high** state one cycle after *mul\_en* goes to **high** state. |
| mul\_stage\_2\_en | Input | *ACCL\_NUM* | Enable the second stage of the multiplier | Goes to **high** state one cycle after *mul\_stage\_1\_en* goes to **high** state. Goes to **low** state one cycle after *mul\_stage\_1\_en* goes to **low** state. |
| mul\_en | Input | *ACCL\_NUM* | Enable the multiplier | Only one bit per cycle can be put to **high**. |
| dsp\_in\_mul\_operands | Input | 2 \* *FU\_NUM* \* *SIMD\_width* | Multiplier inputs | Is 0 before the *mul\_stage\_1\_en* goes to **high** state. |
| dsp\_out\_mul\_results | Output | *FU\_NUM* \* *SIMD\_width* | Multiplier outputs |  |

**Table 2: Multiplier inputs and output signals**

**For all the signals that are replicated per accelerator:**

* Two values from different accelerators can’t change at the same cycle clock.

| Name | Direction | Size | Value |
| ----- | ----- | ----- | ----- |
| clk\_i | Input | 1 | \- |
| rst\_ni | Input | 1 | 1 |
| FUNCT\_SELECT\_MASK | Input | 32 | 32’hFFFFFFFF |
|  MVTYPE\_DSP | Input | 2 | 2’b10 |
| recover\_state\_wires | Input | *1* | 0 |
| halt\_dsp\_lat | Input | *1* | 0 |
| mul\_stage\_1\_en | Input | *1* | 1 |
| mul\_stage\_2\_en | Input | *1* | 1 |
| mul\_en | Input | *1* | 1 |
| dsp\_in\_mul\_operands | Input | 2 \* *1* \* *64* | Free |
| dsp\_out\_mul\_results | Output | *1* \* *64* | \- |

**Table 3: Multiplier constant inputs and output signals (except for the *dsp\_in\_mul\_operands)*  for 1 accelerator with 2 FU operating with 32-bit data**