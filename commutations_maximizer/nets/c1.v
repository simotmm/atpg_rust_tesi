module c1 (A,B,C,D,E);

input A,B;

output E;

wire C,D;

and and1 (C,A,B)
xor xor1 (D,C,B)
or or1(E,C,D)

endmodule