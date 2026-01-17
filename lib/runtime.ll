@dnl = internal constant [4 x i8] c"%d\0A\00"
@d   = internal constant [3 x i8] c"%d\00"	
@s_fmt = internal constant [4 x i8] c"%s\0A\00"
@dealloc = internal constant [9 x i8] c"DEALLOC\0A\00"
@alloc = internal constant [7 x i8] c"ALLOC\0A\00"
@lf  = internal constant [4 x i8] c"%lf\00"	
@errorString = internal constant [15 x i8] c"runtime error\0A\00"

declare i32 @printf(i8*, ...) 
declare i32 @scanf(i8*, ...)
declare i32 @getchar();
declare i32 @puts(i8*)
declare i32 @fgets(i8*, i32, i8*)
declare i8* @calloc(i64, i64)
declare i8* @malloc(i64)
declare void @free(i8*)
declare void @exit(i32)
declare i64 @strlen(i8*)
declare i8* @strcpy(i8*, i8*)
declare i8* @strcat(i8*, i8*)
declare i32 @strcmp(i8*, i8*)

@stdin = external global i8*

define void @printInt(i32 %x) {
   %t0 = getelementptr [4 x i8], [4 x i8]* @dnl, i32 0, i32 0
   call i32 (i8*, ...) @printf(i8* %t0, i32 %x) 
   ret void
}

define void @printString(i8* %s) {
    %fmt = getelementptr [4 x i8], [4 x i8]* @s_fmt, i32 0, i32 0
    call i32 (i8*, ...) @printf(i8* %fmt, i8* %s)
    ret void
}

define void @error() {
	%msg = getelementptr [15 x i8], [15 x i8]* @errorString, i32 0, i32 0
	call i32 (i8*, ...) @printf(i8* %msg)
	call void @exit(i32 1)
	unreachable
}

define i32 @readInt() {
	%res = alloca i32
    %t1 = getelementptr [3 x i8], [3 x i8]* @d, i32 0, i32 0
	call i32 (i8*, ...) @scanf(i8* %t1, i32* %res)
    call i32 @getchar() 
	%t2 = load i32, i32* %res
	ret i32 %t2
}

define i8* @readString() {
    %buffer = call i8* @alloc_string(i64 2048)
    %is_null = icmp eq i8* %buffer, null
    br i1 %is_null, label %err, label %ok
err:
    call void @error()
    unreachable
ok:
    %in = load i8*, i8** @stdin
    %res = call i8* @fgets(i8* %buffer, i32 2048, i8* %in)
    
    %len = call i64 @strlen(i8* %buffer)
    %is_not_empty = icmp ugt i64 %len, 0
    br i1 %is_not_empty, label %trim, label %end

trim:
    %last_idx = sub i64 %len, 1
    %last_char_ptr = getelementptr i8, i8* %buffer, i64 %last_idx
    %last_char = load i8, i8* %last_char_ptr
    %is_nl = icmp eq i8 %last_char, 10
    br i1 %is_nl, label %cut, label %end

cut:
    store i8 0, i8* %last_char_ptr
    br label %end

end:
    ret i8* %buffer
}
 
define i8* @stringConcat(i8* %s1, i8* %s2) {
    %len1 = call i64 @strlen(i8* %s1)
    %len2 = call i64 @strlen(i8* %s2)
    %total_len = add i64 %len1, %len2
    %total_len_with_null = add i64 %total_len, 1
    
    %new_str = call i8* @alloc_string(i64 %total_len_with_null)
    %is_null = icmp eq i8* %new_str, null
    br i1 %is_null, label %err, label %ok
err:
    call void @error()
    unreachable
ok:
    call i8* @strcpy(i8* %new_str, i8* %s1)
    call i8* @strcat(i8* %new_str, i8* %s2)
    ret i8* %new_str
}

define i1 @stringEqual(i8* %s1, i8* %s2) {
    %cmp = call i32 @strcmp(i8* %s1, i8* %s2)
    %res = icmp eq i32 %cmp, 0
    ret i1 %res
}

define i1 @stringNotEqual(i8* %s1, i8* %s2) {
    %cmp = call i32 @strcmp(i8* %s1, i8* %s2)
    %res = icmp ne i32 %cmp, 0
    ret i1 %res
}

define i32 @stringLength(i8* %s) {
    %len = call i64 @strlen(i8* %s)
    %res = trunc i64 %len to i32
    ret i32 %res
}

define i8* @alloc_string(i64 %size) {
    %msg_ptr = getelementptr [7 x i8], [7 x i8]* @alloc, i32 0, i32 0
    call i32 (i8*, ...) @printf(i8* %msg_ptr)
    %total_size = add i64 %size, 8 ; +8 for the reference counter
    %raw = call i8* @malloc(i64 %total_size)
    %is_null = icmp eq i8* %raw, null
    br i1 %is_null, label %err, label %ok
err:
    call void @error()
    unreachable
ok:
    %rc_ptr= bitcast i8* %raw to i64*
    store i64 0, i64* %rc_ptr
    ; Move the pointer by 8 bits so it's a real string pointer.
    %res = getelementptr i8, i8* %raw, i64 8
    ret i8* %res
}

define void @increment_ref(i8* %str) {
    %is_null = icmp eq i8* %str, null
    br i1 %is_null, label %return, label %check_string
check_string:
    %rc_ptr = getelementptr i8, i8* %str, i64 -8 ; Move 8 bits back so we get to the RC.
    %rc_ptr_casted = bitcast i8* %rc_ptr to i64*
    %count = load i64, i64* %rc_ptr_casted
    %is_static = icmp eq i64 %count, -1
    br i1 %is_static, label %return, label %increment
increment:
    %new_count = add i64 %count, 1
    store i64 %new_count, i64* %rc_ptr_casted
    br label %return
return:
    ret void
}

define void @decrement_ref(i8* %str) {
    %is_null = icmp eq i8* %str, null
    br i1 %is_null, label %return, label %check_string
check_string:
    %rc_ptr = getelementptr i8, i8* %str, i64 -8 ; Move 8 bits back so we get to the RC.
    %rc_ptr_casted = bitcast i8* %rc_ptr to i64*
    %count = load i64, i64* %rc_ptr_casted
    %is_static = icmp eq i64 %count, -1
    br i1 %is_static, label %return, label %increment
increment:
    %new_count = sub i64 %count, 1
    store i64 %new_count, i64* %rc_ptr_casted
    %is_zero = icmp eq i64 %new_count, 0
    br i1 %is_zero, label %free, label %return
free:
    %msg_ptr = getelementptr [9 x i8], [9 x i8]* @dealloc, i32 0, i32 0
    call i32 (i8*, ...) @printf(i8* %msg_ptr)
    call void @free(i8* %rc_ptr)
    br label %return
return:
    ret void
}