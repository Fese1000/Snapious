package com.snapious.annotation;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class AnnotationParserTest {

    @Test
    void testGFF3Parsing() {
        String gff3 = "##gff-version 3\n" +
            "chr1\tRefSeq\tgene\t100\t500\t.\t+\t.\tID=gene1;Name=BRCA1\n" +
            "chr1\tRefSeq\texon\t100\t200\t.\t+\t.\tID=exon1;Parent=gene1\n" +
            "chr1\tRefSeq\texon\t300\t500\t.\t+\t.\tID=exon2;Parent=gene1\n" +
            "chr1\tRefSeq\tCDS\t150\t180\t.\t+\t0\tID=cds1;Parent=gene1\n";

        var features = AnnotationParser.parseGFF3(gff3);

        assertFalse(features.isEmpty());
    }

    @Test
    void testGFF3FeatureAttributeParsing() {
        String gff3 = "##gff-version 3\n" +
            "chr1\tRefSeq\tgene\t100\t500\t.\t+\t.\tID=gene1;Name=BRCA1\n";

        var features = AnnotationParser.parseGFF3(gff3);

        assertEquals("gene1", features.get(0).getId());
        assertEquals("BRCA1", features.get(0).getName());
    }

    @Test
    void testGenBankParsing() {
        String genbank = "FEATURES       Location/Qualifiers\n" +
            "             gene            100..500\n" +
            "                     /gene=\"BRCA1\"\n" +
            "             exon            100..200\n" +
            "                     /exon_number=\"1\"\n";

        var features = AnnotationParser.parseGenBank(genbank);

        assertNotNull(features);
    }

    @Test
    void testFeatureTypErkennung() {
        assertEquals(AnnotationParser.FeatureType.GENE,
            AnnotationParser.FeatureType.fromString("gene"));

        assertEquals(AnnotationParser.FeatureType.CDS,
            AnnotationParser.FeatureType.fromString("CDS"));

        assertEquals(AnnotationParser.FeatureType.EXON,
            AnnotationParser.FeatureType.fromString("exon"));
    }

    @Test
    void testFeatureUeberlappung() {
        String gff3 = "##gff-version 3\n" +
            "chr1\tRefSeq\tgene\t100\t500\t.\t+\t.\tID=gene1\n" +
            "chr1\tRefSeq\tgene\t400\t600\t.\t+\t.\tID=gene2\n";

        var features = AnnotationParser.parseGFF3(gff3);

        assertTrue(features.get(0).overlaps(features.get(1)));
    }

    @Test
    void testGeneExtraktion() {
        String gff3 = "##gff-version 3\n" +
            "chr1\tRefSeq\tgene\t100\t500\t.\t+\t.\tID=gene1;Name=BRCA1\n" +
            "chr1\tRefSeq\ttranscript\t100\t500\t.\t+\t.\tID=trans1;Parent=gene1\n" +
            "chr1\tRefSeq\texon\t100\t200\t.\t+\t.\tID=exon1;Parent=trans1\n" +
            "chr1\tRefSeq\texon\t300\t500\t.\t+\t.\tID=exon2;Parent=trans1\n";

        var features = AnnotationParser.parseGFF3(gff3);
        var genes = AnnotationParser.extractGenes(features);

        assertFalse(genes.isEmpty());
    }

    @Test
    void testAnnotationsZusammenfassung() {
        String gff3 = "##gff-version 3\n" +
            "chr1\tRefSeq\tgene\t100\t500\t.\t+\t.\tID=gene1;Name=BRCA1\n" +
            "chr1\tRefSeq\tCDS\t150\t180\t.\t+\t0\tID=cds1;Parent=gene1\n";

        var features = AnnotationParser.parseGFF3(gff3);

        String summary = AnnotationParser.generateAnnotationSummary(features);

        assertNotNull(summary);
        assertTrue(summary.contains("Annotation Summary"));
    }

    @Test
    void testGFF3Export() {
        String gff3 = "##gff-version 3\n" +
            "chr1\tRefSeq\tgene\t100\t500\t.\t+\t.\tID=gene1;Name=BRCA1\n";

        var features = AnnotationParser.parseGFF3(gff3);
        String exported = AnnotationParser.exportToGFF3(features);

        assertNotNull(exported);
        assertTrue(exported.contains("##gff-version 3"));
    }

    @Test
    void testUngueltigeGFF3Zeile() {
        String gff3 = "##gff-version 3\n" +
            "chr1\tRefSeq\tgene\t100\t500\n";  // only 5 columns

        var features = AnnotationParser.parseGFF3(gff3);

        assertTrue(features.isEmpty());
    }

    @Test
    void testUeberlappendeFeaturesFinden() {
        String gff3 = "##gff-version 3\n" +
            "chr1\tRefSeq\tgene\t100\t500\t.\t+\t.\tID=gene1\n" +
            "chr1\tRefSeq\tgene\t400\t600\t.\t+\t.\tID=gene2\n";

        var features = AnnotationParser.parseGFF3(gff3);
        var overlapping = AnnotationParser.findOverlappingFeatures(features);

        assertFalse(overlapping.isEmpty());
    }

    @Test
    void testFullAnnotationReport() {
        String gff3 = "##gff-version 3\n" +
            "chr1\tRefSeq\tgene\t100\t500\t.\t+\t.\tID=gene1;Name=BRCA1\n" +
            "chr1\tRefSeq\ttranscript\t100\t500\t.\t+\t.\tID=trans1;Parent=gene1\n" +
            "chr1\tRefSeq\texon\t100\t200\t.\t+\t.\tID=exon1;Parent=trans1\n" +
            "chr1\tRefSeq\tCDS\t150\t180\t.\t+\t0\tID=cds1;Parent=gene1\n";

        var features = AnnotationParser.parseGFF3(gff3);
        String report = AnnotationParser.generateFullAnnotationReport(features);

        assertNotNull(report);
    }
}
