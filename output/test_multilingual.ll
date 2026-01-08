; ModuleID = 'agn_module'
source_filename = "agn_module"
target triple = "arm64-apple-macosx"

; External declarations
declare i32 @printf(i8*, ...)

; String constants
@.str.fmt.num = private constant [4 x i8] c"%g\0A\00"
@.str.fmt.int = private constant [6 x i8] c"%.0f\0A\00"
@.str.fmt.str = private constant [4 x i8] c"%s\0A\00"

; Main function
define i32 @main() {
entry:
    %X = alloca double, align 8

    store double 0.0, double* %X, align 8
    %tmp0.addr = alloca i64, align 8
    store i64 0, i64* %tmp0.addr, align 8
    br label %loop.header0
loop.header0:
    %tmp1 = load i64, i64* %tmp0.addr, align 8
    %tmp2 = icmp slt i64 %tmp1, 10
    br i1 %tmp2, label %loop.body1, label %loop.end2
loop.body1:
    %tmp3 = load double, double* %X, align 8
    %tmp4 = fadd double %tmp3, 1.0
    store double %tmp4, double* %X, align 8
    %tmp5 = load double, double* %X, align 8
    %tmp6 = fcmp oeq double %tmp5, 5.0
    br i1 %tmp6, label %then3, label %endif5
then3:
    %tmp7 = load double, double* %X, align 8
    %tmp8 = getelementptr [6 x i8], [6 x i8]* @.str.fmt.int, i64 0, i64 0
    call i32 (i8*, ...) @printf(i8* %tmp8, double %tmp7)
    br label %endif5
endif5:
    %tmp9 = add i64 %tmp1, 1
    store i64 %tmp9, i64* %tmp0.addr, align 8
    br label %loop.header0
loop.end2:
    %tmp10 = load double, double* %X, align 8
    %tmp11 = getelementptr [6 x i8], [6 x i8]* @.str.fmt.int, i64 0, i64 0
    call i32 (i8*, ...) @printf(i8* %tmp11, double %tmp10)

    ret i32 0
}
