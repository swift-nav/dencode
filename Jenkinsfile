#!groovy

@Library("ci-jenkins") import com.swiftnav.ci.*

def context = new Context(context: this)
context.setRepo('dencode')

String dockerRunArgs = "\
  -e USER=jenkins"

pipeline {
    agent {
        node {
            label('docker.m')
        }
    }
    options {
        timeout(time: 1, unit: 'HOURS')
        timestamps()
        // Keep builds for 30 days.
        buildDiscarder(logRotator(daysToKeepStr: '30'))
    }
    stages {
        stage('Checks') {
            parallel {
                stage('Check') {
                    agent { dockerfile { reuseNode true; args dockerRunArgs } }
                    steps {
                        gitPrep()
                        script {
                        sh("""#!/bin/bash -ex
                            | cargo check --all-features --all-targets
                            """.stripMargin())
                        }
                    }
                }
                stage('Format') {
                    agent { dockerfile { reuseNode true; args dockerRunArgs } }
                    steps {
                        gitPrep()
                        script {
                        sh("""#!/bin/bash -ex
                            | cargo fmt -- --check
                            """.stripMargin())
                        }
                    }
                }
                stage('Clippy') {
                    agent { dockerfile { reuseNode true; args dockerRunArgs } }
                    steps {
                        gitPrep()
                        script {
                        sh("""#!/bin/bash -ex
                            | cargo clippy -v --all-features --all-targets -- --deny warnings
                            """.stripMargin())
                        }
                    }
                }
                stage('Test') {
                    agent { dockerfile { reuseNode true; args dockerRunArgs } }
                    steps {
                        gitPrep()
                        script {
                        sh("""#!/bin/bash -ex
                            | cargo test --all-features --all-targets
                            """.stripMargin())
                        }
                    }
                }
            }
        }
    }
}
