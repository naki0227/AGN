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
    %Y = alloca double, align 8

    store double 10.0, double* %X, align 8
    store double 20.0, double* %Y, align 8
    %tmp0 = load double, double* %X, align 8
    %tmp1 = load double, double* %Y, align 8
    %tmp2 = fadd double %tmp0, %tmp1
    store double %tmp2, double* %X, align 8
    %tmp3 = load double, double* %X, align 8
    %tmp4 = getelementptr [6 x i8], [6 x i8]* @.str.fmt.int, i64 0, i64 0
    call i32 (i8*, ...) @printf(i8* %tmp4, double %tmp3)

    ret i32 0
}
