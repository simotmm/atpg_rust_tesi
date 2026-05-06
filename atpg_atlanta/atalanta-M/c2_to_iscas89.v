# input A,B,C;
# output X;
# wire D,E;
# and and1(D,A,B);
# not not1(E,C);
# or or1(X,D,E);

INPUT(A)
INPUT(B)
INPUT(C)

OUTPUT(X)

D = AND(A,B)
E = NOT(C)
X = OR(D,E)