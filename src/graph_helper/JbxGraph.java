package dev.telegraphic.jbx.graph;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import org.openrewrite.ExecutionContext;
import org.openrewrite.InMemoryExecutionContext;
import org.openrewrite.SourceFile;
import org.openrewrite.java.JavaIsoVisitor;
import org.openrewrite.java.JavaParser;
import org.openrewrite.java.JavaVisitor;
import org.openrewrite.java.tree.J;
import org.openrewrite.java.tree.JavaType;

public final class JbxGraph {
    private static final Pattern TOKEN = Pattern.compile("(\\w+)=\\\"((?:\\\\.|[^\\\"])*)\\\"");
    private static final String COMPACT_WRAPPER_CLASS = "__JbxCompactSource";

    private JbxGraph() {}

    public static void main(String[] args) throws Exception {
        if (args.length < 2) {
            System.err.println("usage: JbxGraph dump <file.java> | patch <file.java> --expect-graph-hash <hash> --op <op>");
            System.exit(2);
        }
        String command = args[0];
        Path source = Path.of(args[1]);
        if ("dump".equals(command)) {
            boolean json = args.length > 2 && "--json".equals(args[2]);
            System.out.print(dump(source, json));
            return;
        }
        if ("patch".equals(command)) {
            patch(source, args);
            return;
        }
        System.err.println("unknown graph command: " + command);
        System.exit(2);
    }

    private static void patch(Path source, String[] args) throws IOException {
        String expectedHash = null;
        List<String> ops = new ArrayList<>();
        for (int i = 2; i < args.length; i++) {
            if ("--expect-graph-hash".equals(args[i]) && i + 1 < args.length) {
                expectedHash = args[++i];
            } else if ("--op".equals(args[i]) && i + 1 < args.length) {
                ops.add(args[++i]);
            } else {
                System.err.println("unknown graph patch argument: " + args[i]);
                System.exit(2);
            }
        }
        if (expectedHash == null || expectedHash.isBlank()) {
            System.err.println("graph patch requires --expect-graph-hash");
            System.exit(2);
        }
        if (ops.isEmpty()) {
            System.err.println("graph patch requires at least one --op");
            System.exit(2);
        }
        String graph = dump(source, false);
        String actualHash = graphHash(graph);
        if (!expectedHash.equals(actualHash)) {
            System.err.println("graph hash mismatch: expected " + expectedHash + " but was " + actualHash);
            System.exit(1);
        }
        ParsedSource parsed = parse(source);
        SourceFile sourceFile = parsed.sourceFile();
        for (String op : ops) {
            sourceFile = applySetLiteralValue(sourceFile, op);
        }
        String printed = sourceFile.printAll();
        if (parsed.compact()) {
            printed = unwrapCompactSource(printed);
        }
        Files.writeString(source, printed, StandardCharsets.UTF_8);
        System.out.println("patched " + source);
    }

    private static SourceFile applySetLiteralValue(SourceFile sourceFile, String opText) {
        Map<String, String> op = parseOperation(opText);
        if (!"set".equals(op.get("kind"))) {
            throw new IllegalArgumentException("unsupported graph patch operation: " + opText);
        }
        String node = required(op, "node");
        if (node.startsWith("#")) {
            node = node.substring(1);
        }
        String field = required(op, "field");
        if (!"value".equals(field)) {
            throw new IllegalArgumentException("only field=\"value\" is supported for now");
        }
        String expected = required(op, "expect");
        String value = required(op, "value");
        AtomicInteger literalIndex = new AtomicInteger();
        AtomicBoolean changed = new AtomicBoolean();
        String target = node;
        SourceFile updated = (SourceFile) new JavaIsoVisitor<Integer>() {
            @Override
            public J.Literal visitLiteral(J.Literal literal, Integer integer) {
                J.Literal visited = super.visitLiteral(literal, integer);
                String id = "literal-" + literalIndex.incrementAndGet();
                if (!id.equals(target)) {
                    return visited;
                }
                Object literalValue = visited.getValue();
                String old = literalValue == null ? "null" : literalValue.toString();
                if (!expected.equals(old)) {
                    throw new IllegalArgumentException("literal #" + id + " expected value \"" + expected + "\" but was \"" + old + "\"");
                }
                if (visited.getType() != JavaType.Primitive.String) {
                    throw new IllegalArgumentException("literal #" + id + " is not a string literal; graph patch currently supports only string literal values");
                }
                changed.set(true);
                return visited.withValue(value).withValueSource(quoteJava(value));
            }
        }.visit(sourceFile, 0);
        if (!changed.get()) {
            throw new IllegalArgumentException("graph node not found: #" + target);
        }
        return updated;
    }

    private static String dump(Path source, boolean json) throws IOException {
        ParsedSource parsed = parse(source);
        List<Node> nodes = collectNodes(parsed.sourceFile(), parsed.compact());
        String body = graphBody(source, nodes);
        String hash = graphHash(body);
        if (json) {
            return jsonGraph(source, hash, nodes);
        }
        return "jbx-graph v1\n" + "graph-hash " + hash + "\n" + body;
    }

    private static List<Node> collectNodes(SourceFile parsed, boolean compact) {
        List<Node> nodes = new ArrayList<>();
        AtomicInteger classIndex = new AtomicInteger();
        AtomicInteger methodIndex = new AtomicInteger();
        AtomicInteger callIndex = new AtomicInteger();
        AtomicInteger variableIndex = new AtomicInteger();
        AtomicInteger literalIndex = new AtomicInteger();
        new JavaVisitor<Integer>() {
            @Override
            public J visitClassDeclaration(J.ClassDeclaration classDecl, Integer integer) {
                if (!(compact && COMPACT_WRAPPER_CLASS.equals(classDecl.getSimpleName()))) {
                    nodes.add(new Node("class-" + classIndex.incrementAndGet(), "class", "name", classDecl.getSimpleName()));
                }
                return super.visitClassDeclaration(classDecl, integer);
            }

            @Override
            public J visitMethodDeclaration(J.MethodDeclaration method, Integer integer) {
                nodes.add(new Node("method-" + methodIndex.incrementAndGet(), "method", "name", method.getSimpleName()));
                return super.visitMethodDeclaration(method, integer);
            }

            @Override
            public J visitMethodInvocation(J.MethodInvocation method, Integer integer) {
                nodes.add(new Node("call-" + callIndex.incrementAndGet(), "call", "name", method.getSimpleName()));
                return super.visitMethodInvocation(method, integer);
            }

            @Override
            public J visitVariableDeclarations(J.VariableDeclarations multiVariable, Integer integer) {
                for (J.VariableDeclarations.NamedVariable variable : multiVariable.getVariables()) {
                    nodes.add(new Node("variable-" + variableIndex.incrementAndGet(), "variable", "name", variable.getSimpleName()));
                }
                return super.visitVariableDeclarations(multiVariable, integer);
            }

            @Override
            public J visitLiteral(J.Literal literal, Integer integer) {
                Object value = literal.getValue();
                nodes.add(new Node("literal-" + literalIndex.incrementAndGet(), "literal", "value", value == null ? "null" : value.toString()));
                return super.visitLiteral(literal, integer);
            }
        }.visit(parsed, 0);
        return nodes;
    }

    private static String graphBody(Path source, List<Node> nodes) {
        StringBuilder body = new StringBuilder();
        body.append("path ").append(source).append('\n');
        for (Node node : nodes) {
            body.append("node #")
                    .append(node.id())
                    .append(" kind=")
                    .append(node.kind())
                    .append(' ')
                    .append(node.field())
                    .append("=\"")
                    .append(esc(node.value()))
                    .append("\"\n");
        }
        return body.toString();
    }

    private static String jsonGraph(Path source, String hash, List<Node> nodes) {
        StringBuilder json = new StringBuilder();
        json.append("{\n");
        json.append("  \"version\": \"jbx-graph v1\",\n");
        json.append("  \"graphHash\": \"").append(hash).append("\",\n");
        json.append("  \"path\": \"").append(jsonEsc(source.toString())).append("\",\n");
        json.append("  \"nodes\": [\n");
        for (int i = 0; i < nodes.size(); i++) {
            Node node = nodes.get(i);
            json.append("    {\"id\": \"#")
                    .append(jsonEsc(node.id()))
                    .append("\", \"kind\": \"")
                    .append(jsonEsc(node.kind()))
                    .append("\", \"")
                    .append(jsonEsc(node.field()))
                    .append("\": \"")
                    .append(jsonEsc(node.value()))
                    .append("\"}");
            if (i + 1 < nodes.size()) {
                json.append(',');
            }
            json.append('\n');
        }
        json.append("  ]\n");
        json.append("}\n");
        return json.toString();
    }

    private static ParsedSource parse(Path source) throws IOException {
        String text = Files.readString(source, StandardCharsets.UTF_8);
        boolean compact = isCompactSource(text);
        String parseText = compact ? wrapCompactSource(text) : text;
        ExecutionContext ctx = new InMemoryExecutionContext(throwable -> {
            throw new RuntimeException(throwable);
        });
        ctx.putMessage(ExecutionContext.REQUIRE_PRINT_EQUALS_INPUT, false);
        SourceFile sourceFile = JavaParser.fromJavaVersion()
                .build()
                .parse(ctx, parseText)
                .findFirst()
                .orElseThrow(() -> new IllegalArgumentException("OpenRewrite did not parse " + source));
        if (sourceFile instanceof org.openrewrite.tree.ParseError parseError) {
            throw parseError.toException();
        }
        if (!(sourceFile instanceof J.CompilationUnit)) {
            throw new IllegalArgumentException("OpenRewrite parsed " + source + " as " + sourceFile.getClass().getName() + " instead of a Java compilation unit");
        }
        return new ParsedSource(sourceFile, compact);
    }

    private static String graphHash(String graph) {
        return sha256(graph.replaceFirst("(?s)^jbx-graph v1\\ngraph-hash [0-9a-f]+\\n", ""));
    }

    private static String sha256(String value) {
        try {
            java.security.MessageDigest digest = java.security.MessageDigest.getInstance("SHA-256");
            byte[] bytes = digest.digest(value.getBytes(StandardCharsets.UTF_8));
            StringBuilder hex = new StringBuilder(bytes.length * 2);
            for (byte b : bytes) {
                hex.append(String.format("%02x", b));
            }
            return hex.toString();
        } catch (java.security.NoSuchAlgorithmException e) {
            throw new IllegalStateException(e);
        }
    }

    private static Map<String, String> parseOperation(String text) {
        Map<String, String> result = new LinkedHashMap<>();
        String trimmed = text.trim();
        int firstSpace = trimmed.indexOf(' ');
        result.put("kind", firstSpace < 0 ? trimmed : trimmed.substring(0, firstSpace));
        Matcher matcher = TOKEN.matcher(trimmed);
        while (matcher.find()) {
            result.put(matcher.group(1), unescape(matcher.group(2)));
        }
        return result;
    }

    private static String required(Map<String, String> op, String name) {
        String value = op.get(name);
        if (value == null) {
            throw new IllegalArgumentException("operation missing " + name);
        }
        return value;
    }

    private static String esc(String value) {
        return value.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n").replace("\r", "\\r").replace("\t", "\\t");
    }

    private static String unescape(String value) {
        StringBuilder out = new StringBuilder();
        boolean slash = false;
        for (int i = 0; i < value.length(); i++) {
            char ch = value.charAt(i);
            if (slash) {
                out.append(switch (ch) {
                    case 'n' -> '\n';
                    case 'r' -> '\r';
                    case 't' -> '\t';
                    default -> ch;
                });
                slash = false;
            } else if (ch == '\\') {
                slash = true;
            } else {
                out.append(ch);
            }
        }
        if (slash) {
            out.append('\\');
        }
        return out.toString();
    }

    private static String jsonEsc(String value) {
        return value.replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("\n", "\\n")
                .replace("\r", "\\r")
                .replace("\t", "\\t");
    }

    private static boolean isCompactSource(String source) {
        int braceDepth = 0;
        boolean inBlockComment = false;
        for (String line : source.split("\\R")) {
            String trimmed = line.stripLeading();
            if (braceDepth == 0) {
                if (inBlockComment || trimmed.startsWith("/*")) {
                    inBlockComment = !trimmed.contains("*/");
                    continue;
                }
                if (isIgnorableTopLevelPrefix(trimmed) || trimmed.startsWith("@")) {
                    continue;
                }
                return !startsWithJavaTypeDeclaration(trimmed);
            }
            braceDepth = updateBraceDepth(braceDepth, line);
        }
        return false;
    }

    private static boolean isIgnorableTopLevelPrefix(String trimmed) {
        return trimmed.isEmpty()
                || trimmed.startsWith("//")
                || trimmed.startsWith("#!")
                || trimmed.startsWith("package ")
                || trimmed.startsWith("import ");
    }

    private static int updateBraceDepth(int depth, String line) {
        int next = depth;
        boolean inString = false;
        boolean inChar = false;
        for (int i = 0; i < line.length(); i++) {
            char ch = line.charAt(i);
            if (ch == '\\' && (inString || inChar)) {
                i++;
                continue;
            }
            if (ch == '"' && !inChar) {
                inString = !inString;
                continue;
            }
            if (ch == '\'' && !inString) {
                inChar = !inChar;
                continue;
            }
            if (inString || inChar) {
                continue;
            }
            if (ch == '/' && i + 1 < line.length() && line.charAt(i + 1) == '/') {
                break;
            }
            if (ch == '{') {
                next++;
            } else if (ch == '}') {
                next = Math.max(0, next - 1);
            }
        }
        return next;
    }

    private static boolean startsWithJavaTypeDeclaration(String trimmed) {
        String[] prefixes = {
                "class ", "abstract class ", "sealed class ", "non-sealed class ", "final class ",
                "public class ", "public abstract class ", "public sealed class ", "public non-sealed class ", "public final class ",
                "record ", "public record ", "interface ", "public interface ", "enum ", "public enum ", "@interface ", "public @interface "
        };
        for (String prefix : prefixes) {
            if (trimmed.startsWith(prefix)) {
                return true;
            }
        }
        return false;
    }

    private static String wrapCompactSource(String source) {
        String[] split = splitCompactPrefix(source);
        StringBuilder wrapped = new StringBuilder();
        wrapped.append(split[0]);
        wrapped.append("class ").append(COMPACT_WRAPPER_CLASS).append(" {\n");
        for (String line : split[1].split("\\R", -1)) {
            if (!line.isEmpty()) {
                wrapped.append("    ").append(line);
            }
            wrapped.append('\n');
        }
        wrapped.append("}\n");
        return wrapped.toString();
    }

    private static String[] splitCompactPrefix(String source) {
        StringBuilder prefix = new StringBuilder();
        StringBuilder body = new StringBuilder();
        boolean inPrefix = true;
        boolean inBlockComment = false;
        for (String line : source.split("\\R", -1)) {
            String trimmed = line.stripLeading();
            if (inPrefix) {
                boolean prefixLine = inBlockComment
                        || trimmed.isEmpty()
                        || trimmed.startsWith("//")
                        || trimmed.startsWith("#!")
                        || trimmed.startsWith("package ")
                        || trimmed.startsWith("import ")
                        || trimmed.startsWith("/*");
                if (prefixLine) {
                    prefix.append(line).append('\n');
                    if (inBlockComment || trimmed.startsWith("/*")) {
                        inBlockComment = !trimmed.contains("*/");
                    }
                    continue;
                }
                inPrefix = false;
            }
            body.append(line).append('\n');
        }
        return new String[] {prefix.toString(), body.toString().stripTrailing()};
    }

    private static String unwrapCompactSource(String printed) {
        String[] lines = printed.split("\\R", -1);
        int wrapperStart = -1;
        for (int i = 0; i < lines.length; i++) {
            if (lines[i].trim().equals("class " + COMPACT_WRAPPER_CLASS + " {")) {
                wrapperStart = i;
                break;
            }
        }
        if (wrapperStart < 0) {
            return printed;
        }
        int wrapperEnd = -1;
        for (int i = lines.length - 1; i > wrapperStart; i--) {
            if (lines[i].trim().equals("}")) {
                wrapperEnd = i;
                break;
            }
        }
        if (wrapperEnd < 0) {
            return printed;
        }
        StringBuilder out = new StringBuilder();
        for (int i = 0; i < wrapperStart; i++) {
            out.append(lines[i]).append('\n');
        }
        int indent = Integer.MAX_VALUE;
        for (int i = wrapperStart + 1; i < wrapperEnd; i++) {
            String line = lines[i];
            if (!line.isBlank()) {
                indent = Math.min(indent, line.length() - line.stripLeading().length());
            }
        }
        if (indent == Integer.MAX_VALUE) {
            indent = 0;
        }
        for (int i = wrapperStart + 1; i < wrapperEnd; i++) {
            String line = lines[i];
            out.append(line.length() >= indent ? line.substring(indent) : line).append('\n');
        }
        return out.toString();
    }

    private static String quoteJava(String value) {
        return "\"" + esc(value).replace("\t", "\\t") + "\"";
    }

    private record ParsedSource(SourceFile sourceFile, boolean compact) {}

    private record Node(String id, String kind, String field, String value) {}
}
