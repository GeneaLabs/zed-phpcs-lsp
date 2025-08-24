<?php
// Missing file-level docblock (PSR12.Files.FileHeader)

namespace MyNamespace; // Namespace declaration issues
 dfd
// Informational severity warnings
$variable_with_underscores = 'test'; // Variable naming convention (camelCase preferred)
define('LEGACY_CONSTANT', 'value'); // Using define() instead of const (informational)
$array = array(); // Old array syntax (informational)
use DateTime;
use     Exception; // Multiple spaces after use keyword
use DateTimeZone ,  DateTimeInterface; // Comma-separated imports not allowed

// Missing blank line after use statements
class   MyClass   { // Multiple spaces around class name

    // Property without visibility
    $undeclaredProperty;

    // Properties with various issues
    public $publicProperty  =  'value'; // Spacing around assignment
    private$privateProperty='test'; // No spacing
    protected  static   $staticProperty; // Multiple spaces

    // Constants with issues
    const MY_CONSTANT='value'; // No spaces around assignment
    public const    ANOTHER_CONSTANT  =   42; // Multiple spaces

    // Method with multiple issues
    function badMethod(){echo "bad";} // Missing visibility, spacing issues

    public function __construct( $param1,$param2 ) // Spacing issues in parameters
    {
        // Control structure issues
        if($param1==true){ // No space after if, spacing around operator
            echo"Hello"; // No space after echo
        }
        else{ // Else on same line as closing brace
            echo  "World"; // Multiple spaces
        }

        // More control structure issues
        for($i=0;$i<10;$i++){ // No spaces in for loop
            if($i==5)echo $i; // Single line if without braces
        }

        while($param2>0) // No space after while
        {
            $param2 --;  // Space before decrement
            break ; // Space before semicolon
        }

        // Switch statement issues
        switch($param1){
        case 1: // Wrong indentation
        echo "one";
        break;
        case 2:echo "two";break; // Multiple statements on one line
        default :  // Space before colon
            echo "default"  ; // Space before semicolon
        }

        // Ternary operator issues
        $result=$param1?'yes':'no'; // No spaces around operators
        $another  =  $param2  ?  'true'  :  'false'; // Too many spaces

        // Array issues
        $array=array(1,2,3); // Old array syntax, no spaces
        $newArray  =  [  1  ,  2  ,  3  ]; // Too many spaces
        $assocArray=['key'=>'value','another'=>'test']; // No spaces

        // String concatenation issues
        $string='Hello'.'World'; // No spaces around concatenation
        $another  =  'Test'  .  'String'; // Too many spaces

        // Logical operator issues
        if($param1&&$param2){ // No spaces around &&
            echo "both";
        }
        if($param1  ||  $param2){ // Too many spaces around ||
            echo "either";
        }

        // Comparison operator issues
        if($param1!=$param2){ // No spaces around !=
            if($param1>=$param2){ // No spaces around >=
                if($param1<=$param2){ // No spaces around <=
                    echo "impossible";
                }
            }
        }

        // Assignment operator issues
        $x=5; // No spaces
        $y  =  10; // Too many spaces
        $x+=$y; // No spaces around +=
        $x  -=  $y; // Too many spaces around -=
        $x*=$y; // No spaces around *=
        $x/=$y; // No spaces around /=
        $x%=$y; // No spaces around %=

        // Bitwise operator issues
        $a=$param1&$param2; // No spaces around &
        $b=$param1|$param2; // No spaces around |
        $c=$param1^$param2; // No spaces around ^
        $d=~$param1; // No space after ~
        $e=$param1<<2; // No spaces around <<
        $f=$param1>>2; // No spaces around >>

        // Type casting issues
        $int=(int)$param1; // No space after cast
        $string  =  (string)  $param2; // Too many spaces
        $bool=(bool)$param1; // No space
        $float = (  float  ) $param2; // Spaces inside cast

        // Function call issues
        print("test"); // Function call spacing
        echo("another"); // Echo with parentheses
        isset  (  $var  ); // Spacing issues
        empty($var) ; // Space before semicolon

        // Increment/decrement issues
        $i ++ ; // Spaces around increment
        ++ $j; // Space after prefix increment
        $k-- ; // Space before semicolon
        -- $m ; // Spaces around decrement
    }

    // Method with too many parameters
    public function tooManyParams($a,$b,$c,$d,$e,$f,$g,$h,$i,$j,$k,$l,$m,$n,$o,$p)
    {
        return$a+$b+$c+$d+$e+$f+$g+$h+$i+$j+$k+$l+$m+$n+$o+$p; // No space after return
    }

    // Method with inconsistent spacing
    public  static  function   staticMethod  (  )  {
        // Empty method with spacing issues
    }

    // Method with missing return type hint spacing
    public function returnType():string{ // No space before return type
        return'test'; // No space after return
    }

    // Method with nullable type hint issues
    public function nullable(? string $param):?int{ // Space after ?, no space before ?
        return null ;
    }

    // Anonymous function issues
    public function anonymousFunction()
    {
        $closure=function($x)use($y){ // No spaces
            return$x+$y;
        };

        $arrow  =  fn  (  $x  )  =>  $x  *  2; // Too many spaces

        // Callback issues
        array_map(function($item){return$item*2;},$array); // Multiple issues
    }

    // Try-catch issues
    public function exceptions()
    {
        try{
            throw new Exception('test');
        }catch(Exception $e){ // No space after catch
            echo$e->getMessage(); // No space after echo
        }finally{ // No space after finally
            echo"cleanup"; // No space after echo
        }

        // Multiple catch blocks
        try {
            // code
        } catch (Exception $e) {
            // handle
        }catch(RuntimeException $e){ // No space before catch
            // handle
        }
    }

    // Foreach issues
    public function loops()
    {
        foreach($array as$key=>$value){ // No spaces
            echo$key.':'.$value; // Multiple spacing issues
        }

        foreach  (  $array  as  $item  ) { // Too many spaces
            continue ; // Space before semicolon
        }

        // Do-while issues
        do{
            $x++;
        }while($x<10); // No space after while
    }

    // Visibility order issues (should be public, protected, private)
    private function privateMethod() {}
    public function publicMethod() {}
    protected function protectedMethod() {}

    // Line length issues - this is a very long line that exceeds the recommended maximum line length and should trigger a warning about line length in PSR12 standard
    public function veryLongMethodNameThatExceedsRecommendedLineLengthAndShouldTriggerWarning($parameterOne, $parameterTwo, $parameterThree, $parameterFour)
    {
        // Very long line in method body
        $veryLongVariableNameThatIsUsedToTestLineLengthIssues = "This is a very long string that when combined with the variable name will exceed the maximum recommended line length according to PSR12 coding standards";
    }
}

// Interface issues
interface  MyInterface  { // Spacing issues
    public function method($param) ; // Space before semicolon
}

// Trait issues
trait  MyTrait  {
    use  AnotherTrait  ; // Spacing issues

    abstract  public  function  abstractMethod(); // Spacing issues
}

// Global function issues
function globalFunction  (  $param  )  {
    global$globalVar; // No space after global
    static$staticVar; // No space after static

    // Goto issues (generally discouraged)
    goto label;
    label:
    echo"jumped"; // No space
}

// Multiple classes in single file (PSR12 violation)
class SecondClass {
    // Class content
}

// More informational issues
@error_reporting(E_ALL); // Using @ error suppression (informational warning)
$unused_variable = 'never used'; // Unused variable (informational)
/* TODO: This is a todo comment that should be addressed */ // TODO comment (informational)
eval('$x = 1;'); // Using eval() is discouraged (informational)

// Closing tag present (should be omitted)
?>
