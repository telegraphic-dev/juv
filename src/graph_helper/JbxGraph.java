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

    private JbxGraph() {}

    public static void main(String[] args) throws Exception {
        if (args.length < 2) {
            System.err.println("usage: JbxGraph dump <file.java> | patch <file.java> --expect-graph-hash <hash> --op <op>");
            System.exit(2);
        }
        String command = args[0];
        Path source = Path.of(args[1]);
        if ("dump".equals(command)) {
            System.out.print(dump(source));
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
        String graph = dump(source);
        String actualHash = graphHash(graph);
        if (!expectedHash.equals(actualHash)) {
            System.err.println("graph hash mismatch: expected " + expectedHash + " but was " + actualHash);
            System.exit(1);
        }
        SourceFile parsed = parse(source);
        for (String op : ops) {
            parsed = applySetLiteralValue(parsed, op);
        }
        Files.writeString(source, parsed.printAll(), StandardCharsets.UTF_8);
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

    private static String dump(Path source) throws IOException {
        SourceFile parsed = parse(source);
        List<String> nodes = new ArrayList<>();
        AtomicInteger classIndex = new AtomicInteger();
        AtomicInteger methodIndex = new AtomicInteger();
        AtomicInteger callIndex = new AtomicInteger();
        AtomicInteger variableIndex = new AtomicInteger();
        AtomicInteger literalIndex = new AtomicInteger();
        new JavaVisitor<Integer>() {
            @Override
            public J visitClassDeclaration(J.ClassDeclaration classDecl, Integer integer) {
                nodes.add("node #class-" + classIndex.incrementAndGet() + " kind=class name=\"" + esc(classDecl.getSimpleName()) + "\"");
                return super.visitClassDeclaration(classDecl, integer);
            }

            @Override
            public J visitMethodDeclaration(J.MethodDeclaration method, Integer integer) {
                nodes.add("node #method-" + methodIndex.incrementAndGet() + " kind=method name=\"" + esc(method.getSimpleName()) + "\"");
                return super.visitMethodDeclaration(method, integer);
            }

            @Override
            public J visitMethodInvocation(J.MethodInvocation method, Integer integer) {
                nodes.add("node #call-" + callIndex.incrementAndGet() + " kind=call name=\"" + esc(method.getSimpleName()) + "\"");
                return super.visitMethodInvocation(method, integer);
            }

            @Override
            public J visitVariableDeclarations(J.VariableDeclarations multiVariable, Integer integer) {
                for (J.VariableDeclarations.NamedVariable variable : multiVariable.getVariables()) {
                    nodes.add("node #variable-" + variableIndex.incrementAndGet() + " kind=variable name=\"" + esc(variable.getSimpleName()) + "\"");
                }
                return super.visitVariableDeclarations(multiVariable, integer);
            }

            @Override
            public J visitLiteral(J.Literal literal, Integer integer) {
                Object value = literal.getValue();
                nodes.add("node #literal-" + literalIndex.incrementAndGet() + " kind=literal value=\"" + esc(value == null ? "null" : value.toString()) + "\"");
                return super.visitLiteral(literal, integer);
            }
        }.visit(parsed, 0);
        StringBuilder body = new StringBuilder();
        body.append("path ").append(source).append('\n');
        for (String node : nodes) {
            body.append(node).append('\n');
        }
        String hash = graphHash(body.toString());
        return "jbx-graph v1\n" + "graph-hash " + hash + "\n" + body;
    }

    private static SourceFile parse(Path source) throws IOException {
        ExecutionContext ctx = new InMemoryExecutionContext(Throwable::printStackTrace);
        String text = Files.readString(source, StandardCharsets.UTF_8);
        return JavaParser.fromJavaVersion()
                .build()
                .parse(ctx, text)
                .findFirst()
                .orElseThrow(() -> new IllegalArgumentException("OpenRewrite did not parse " + source));
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

    private static String quoteJava(String value) {
        return "\"" + esc(value).replace("\t", "\\t") + "\"";
    }
}
